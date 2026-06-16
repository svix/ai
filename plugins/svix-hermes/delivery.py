"""Copy of the webhook delivery functions from Hermes' built-in webhook
adapter (``gateway/platforms/webhook.py``).

The functions below are copied from the upstream adapter and packaged as a mixin so 
the Svix plugin installs into a stock Hermes checkout without modifying it. 
Plugin-specific deviations (the ``[svix]`` log tag, ``{__event__}`` placeholder, unresolved
deliver_extra placeholder warnings, and the extracted ``_inject_skill``
helper) are kept minimal so the file stays diffable against upstream.
"""

import asyncio
import json
import logging
import re
import subprocess
from typing import Any, Dict, Optional

logger = logging.getLogger(__name__)

_PLACEHOLDER_RE = re.compile(r"\{([a-zA-Z0-9_.]+)\}")

_LOG_TAG = "[svix]"


class WebhookDeliveryMixin:
    """Render → deliver pipeline copied from the built-in webhook adapter.

    The concrete adapter must set ``_delivery_info`` / ``_delivery_info_created``
    / ``_delivery_info_ttl`` in ``__init__`` and inherit ``BasePlatformAdapter``.
    Mix in *before* ``BasePlatformAdapter`` so this ``send()`` satisfies the
    abstract method.
    """

    _delivery_info: Dict[str, dict]
    _delivery_info_created: Dict[str, float]
    _delivery_info_ttl: float

    # ------------------------------------------------------------------
    # Prompt rendering
    # ------------------------------------------------------------------

    def _render_prompt(
        self,
        template: str,
        payload: dict,
        event_type: str,
        route_name: str,
    ) -> str:
        """Render a prompt template with the webhook payload.

        Supports dot-notation access into nested dicts:
        ``{pull_request.title}`` → ``payload["pull_request"]["title"]``

        Special token ``{__raw__}`` dumps the entire payload as indented
        JSON (truncated to 4000 chars).  Useful for monitoring alerts or
        any webhook where the agent needs to see the full payload.
        """
        if not template:
            truncated = json.dumps(payload, indent=2)[:4000]
            return (
                f"Svix event '{event_type}' on route "
                f"'{route_name}':\n\n```json\n{truncated}\n```"
            )

        def _resolve(match: "re.Match") -> str:
            key = match.group(1)
            # Special token: dump the entire payload as JSON
            if key == "__raw__":
                return json.dumps(payload, indent=2)[:4000]
            if key == "__event__":
                return event_type
            value: Any = payload
            for part in key.split("."):
                if isinstance(value, dict):
                    value = value.get(part, f"{{{key}}}")
                else:
                    return f"{{{key}}}"
            if isinstance(value, (dict, list)):
                return json.dumps(value, indent=2)[:2000]
            return str(value)

        return _PLACEHOLDER_RE.sub(_resolve, template)

    def _render_delivery_extra(
        self, extra: dict, payload: dict, event_type: str = ""
    ) -> dict:
        """Render delivery_extra template values with payload data.

        ``event_type`` is threaded through so ``{__event__}`` resolves in
        deliver_extra values (e.g. a per-event chat_id), matching its support
        in prompts. (Upstream passes ``""`` here; the plugin documents
        ``{__event__}`` so we wire the real value.)
        """
        rendered: Dict[str, Any] = {}
        for key, value in extra.items():
            if isinstance(value, str):
                out = self._render_prompt(value, payload, event_type, "")
                unresolved = [
                    m.group(0)
                    for m in _PLACEHOLDER_RE.finditer(value)
                    if m.group(0) in out
                ]
                if unresolved:
                    logger.warning(
                        "%s deliver_extra.%s still contains unresolved "
                        "placeholder(s) %s after rendering (payload path "
                        "missing?); delivery may be mis-routed. Rendered "
                        "value: %r",
                        _LOG_TAG, key, unresolved, out,
                    )
                rendered[key] = out
            else:
                rendered[key] = value
        return rendered

    def _inject_skill(self, prompt: str, skills) -> str:
        """Inject skill content if configured.

        We call build_skill_invocation_message() directly rather than
        using /skill-name slash commands — the gateway's command parser
        would intercept those and break the flow.
        """
        if not skills:
            return prompt
        try:
            from agent.skill_commands import (
                build_skill_invocation_message,
                get_skill_commands,
            )

            skill_cmds = get_skill_commands()
            for skill_name in skills:
                cmd_key = f"/{skill_name}"
                if cmd_key in skill_cmds:
                    skill_content = build_skill_invocation_message(
                        cmd_key, user_instruction=prompt
                    )
                    if skill_content:
                        return skill_content
                else:
                    logger.warning(
                        "%s Skill '%s' not found", _LOG_TAG, skill_name
                    )
        except Exception as e:
            logger.warning("%s Skill loading failed: %s", _LOG_TAG, e)
        return prompt

    # ------------------------------------------------------------------
    # delivery_info bookkeeping
    # ------------------------------------------------------------------

    def _prune_delivery_info(self, now: float) -> None:
        """Drop delivery_info entries older than the TTL.

        Called once per polled event that dispatches an agent run, so the dict
        stays bounded even if many events fire and never receive a final
        response (e.g. crashed mid-run).
        """
        cutoff = now - self._delivery_info_ttl
        stale = [
            k
            for k, t in self._delivery_info_created.items()
            if t < cutoff
        ]
        for k in stale:
            self._delivery_info.pop(k, None)
            self._delivery_info_created.pop(k, None)

    # ------------------------------------------------------------------
    # Response delivery
    # ------------------------------------------------------------------

    async def send(
        self,
        chat_id: str,
        content: str,
        reply_to: Optional[str] = None,
        metadata: Optional[Dict[str, Any]] = None,
    ) -> "SendResult":
        """Deliver the agent's response to the configured destination.

        chat_id is ``svix:{route}:{delivery_id}``.  The delivery info stored
        when the event was polled is read with ``.get()`` (not popped) so that
        interim status messages emitted before the final response — fallback-
        model notifications, context-pressure warnings, etc. — do not consume
        the entry and silently downgrade the final response to the ``log``
        deliver type.  TTL cleanup happens when each event is processed.
        """
        from gateway.platforms.base import SendResult

        delivery = self._delivery_info.get(chat_id, {})
        deliver_type = delivery.get("deliver", "log")

        if deliver_type == "log":
            logger.info(
                "%s Response for %s: %s", _LOG_TAG, chat_id, content[:200]
            )
            return SendResult(success=True)

        if deliver_type == "github_comment":
            return await self._deliver_github_comment(content, delivery)

        # Cross-platform delivery — any platform with a gateway adapter.
        if not self.gateway_runner:
            logger.warning(
                "%s No gateway runner for deliver type: %s",
                _LOG_TAG, deliver_type,
            )
            return SendResult(
                success=False,
                error="No gateway runner for cross-platform delivery",
            )
        return await self._deliver_cross_platform(deliver_type, content, delivery)

    async def _direct_deliver(self, content: str, delivery: dict) -> "SendResult":
        """Deliver *content* directly without invoking the agent.

        Used by ``deliver_only`` routes: the rendered template becomes the
        literal message body, and we dispatch to the same delivery helpers
        that the agent-mode ``send()`` flow uses.  All target types that
        work in agent mode work here — Telegram, Discord, Slack, GitHub
        PR comments, etc.
        """
        from gateway.platforms.base import SendResult

        deliver_type = delivery.get("deliver", "log")

        if deliver_type == "log":
            # Shouldn't reach here — startup validation rejects deliver_only
            # with deliver=log — but guard defensively.
            logger.info(
                "%s direct-deliver log-only: %s", _LOG_TAG, content[:200]
            )
            return SendResult(success=True)

        if deliver_type == "github_comment":
            return await self._deliver_github_comment(content, delivery)

        # Fall through to the cross-platform dispatcher, which validates the
        # target name and routes via the gateway runner.
        return await self._deliver_cross_platform(deliver_type, content, delivery)

    async def _deliver_github_comment(
        self, content: str, delivery: dict
    ) -> "SendResult":
        """Post agent response as a GitHub PR/issue comment via ``gh`` CLI."""
        from gateway.platforms.base import SendResult

        extra = delivery.get("deliver_extra", {})
        repo = extra.get("repo", "")
        pr_number = extra.get("pr_number", "")

        if not repo or not pr_number:
            logger.error(
                "%s github_comment delivery missing repo or pr_number",
                _LOG_TAG,
            )
            return SendResult(success=False, error="Missing repo or pr_number")

        try:
            # Deviation from upstream (webhook.py calls subprocess.run directly):
            # uses asyncio to avoid blocking the loop because of a slow gh cli call
            result = await asyncio.to_thread(
                subprocess.run,
                [
                    "gh", "pr", "comment", str(pr_number),
                    "--repo", repo, "--body", content,
                ],
                capture_output=True, text=True, timeout=30,
            )
            if result.returncode == 0:
                logger.info(
                    "%s Posted comment on %s#%s", _LOG_TAG, repo, pr_number
                )
                return SendResult(success=True)
            logger.error(
                "%s gh pr comment failed: %s", _LOG_TAG, result.stderr
            )
            return SendResult(success=False, error=result.stderr)
        except FileNotFoundError:
            logger.error(
                "%s 'gh' CLI not found — install GitHub CLI for "
                "github_comment delivery", _LOG_TAG,
            )
            return SendResult(success=False, error="gh CLI not installed")
        except Exception as e:
            logger.error(
                "%s github_comment delivery error: %s", _LOG_TAG, e
            )
            return SendResult(success=False, error=str(e))

    async def _deliver_cross_platform(
        self, platform_name: str, content: str, delivery: dict
    ) -> "SendResult":
        """Route response to another platform (telegram, discord, etc.)."""
        from gateway.config import Platform
        from gateway.platforms.base import SendResult

        if not self.gateway_runner:
            return SendResult(
                success=False,
                error="No gateway runner for cross-platform delivery",
            )

        try:
            target_platform = Platform(platform_name)
        except ValueError:
            return SendResult(
                success=False, error=f"Unknown platform: {platform_name}"
            )

        adapter = self.gateway_runner.adapters.get(target_platform)
        if not adapter:
            return SendResult(
                success=False, error=f"Platform {platform_name} not connected"
            )

        # Use home channel if no specific chat_id in deliver_extra
        extra = delivery.get("deliver_extra", {})
        chat_id = extra.get("chat_id", "")
        if not chat_id:
            home = self.gateway_runner.config.get_home_channel(target_platform)
            if home:
                chat_id = home.chat_id
            else:
                return SendResult(
                    success=False,
                    error=f"No chat_id or home channel for {platform_name}",
                )

        # Pass thread_id from deliver_extra so Telegram forum topics work
        metadata = None
        thread_id = extra.get("message_thread_id") or extra.get("thread_id")
        if thread_id:
            metadata = {"thread_id": thread_id}

        return await adapter.send(chat_id, content, metadata=metadata)
