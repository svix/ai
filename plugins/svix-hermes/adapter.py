"""Svix polling platform adapter (Hermes Agent plugin).

Polls Svix's polling-endpoint API for webhook events instead of hosting an
HTTP server. See README.md for configuration, route fields, and design notes.
"""

import asyncio
import logging
import os
import re
import time
from typing import Any, Dict, Optional
from urllib.parse import urlparse

from gateway.config import Platform, PlatformConfig
from gateway.platforms.base import (
    BasePlatformAdapter,
    MessageEvent,
    MessageType,
)

from .delivery import WebhookDeliveryMixin

logger = logging.getLogger(__name__)

_DEFAULT_POLL_INTERVAL = 5.0
_DEFAULT_POLL_LIMIT = 50
# Cap on concurrent agent runs across all routes. Polling gives natural
# backpressure: the poll loop awaits a free slot before dispatching, so a
# large backlog drains at bounded concurrency instead of spawning one agent
# run per message. This is the polling-world analogue of the upstream webhook
# adapter's per-route rate limit (``webhook.py`` ``rate_limit``, default 30/min).
_DEFAULT_MAX_CONCURRENT = 5
# Floor sleep when the server returns done=False with an empty page, so a
# misbehaving server can't spin the poll loop against the API.
_EMPTY_PAGE_SLEEP = 0.5
_DEDUP_TTL = 3600.0
# Truthy spellings, matching the host's shared TRUTHY_STRINGS (incl. "on").
_TRUTHY = {"1", "true", "yes", "on"}
# Polling-endpoint URL shape: /api/v1/app/{app_id}/poller/{sink_id}/
_URL_PATH_RE = re.compile(
    r"/api/v\d+/app/(?P<app_id>[^/]+)/poller/(?P<sink_id>[^/]+)/?",
)

# Injected into the agent's system prompt for Svix-sourced turns via the
# registry entry's ``platform_hint`` field.
_SVIX_PLATFORM_HINT = (
    "You are processing a webhook event delivered via Svix's polling "
    "endpoint API. There is no user present in this turn — execute the "
    "task fully and autonomously based on the rendered event payload, "
    "making reasonable decisions where needed. Your response is forwarded "
    "to the route's configured deliver target (telegram, discord, "
    "github_comment, etc.), so put the final user-facing content directly "
    "in your response. Format for the destination platform; when in doubt, "
    "prefer concise plain text."
)


def _import_svix():
    """Return (SvixAsync, SvixOptions, MessagePollerConsumerPollOptions) or None."""
    try:
        from svix.api import (
            MessagePollerConsumerPollOptions,
            SvixAsync,
            SvixOptions,
        )
        return SvixAsync, SvixOptions, MessagePollerConsumerPollOptions
    except ImportError:
        return None


def check_svix_requirements() -> bool:
    """Return True when the Svix SDK is importable. Per-route auth tokens are
    validated later, at ``connect()`` time.

    No auto-install: plugins can't register a ``tools.lazy_deps`` feature, so
    there is nothing to install through. The ``install_hint`` on the registry
    entry surfaces ``pip install svix`` when the SDK is missing.
    """
    return _import_svix() is not None


def _parse_polling_url(url: str) -> tuple[str, str, Optional[str]]:
    """Split a polling URL into (app_id, sink_id, server_url).

    server_url is the scheme+host (None for a relative URL); carrying it lets
    region-specific / self-hosted deployments work without an extra config knob.
    """
    parsed = urlparse(url)
    match = _URL_PATH_RE.search(parsed.path or "")
    if not match:
        raise ValueError(
            f"Invalid Svix polling URL: {url!r}. Expected "
            f"https://<svix-host>/api/v1/app/<app_id>/poller/<sink_id>/"
        )
    server_url = None
    if parsed.scheme and parsed.netloc:
        server_url = f"{parsed.scheme}://{parsed.netloc}"
    return match.group("app_id"), match.group("sink_id"), server_url


class SvixAdapter(WebhookDeliveryMixin, BasePlatformAdapter):
    """Polls Svix polling endpoints and dispatches events to the gateway."""

    def __init__(self, config: PlatformConfig):
        # ``Platform("svix")`` resolves via ``Platform._missing_`` because this
        # platform is registry-registered — no core enum member required.
        super().__init__(config, Platform("svix"))
        self._poll_interval: float = float(
            config.extra.get("poll_interval", _DEFAULT_POLL_INTERVAL)
        ) or _DEFAULT_POLL_INTERVAL
        self._poll_limit: int = int(
            config.extra.get("poll_limit", _DEFAULT_POLL_LIMIT)
        ) or _DEFAULT_POLL_LIMIT
        self._routes: Dict[str, dict] = config.extra.get("routes", {}) or {}

        # Bound concurrent agent runs so a backlog drain can't fan out one run
        # per message. The poll loop awaits this before dispatching, so polling
        # itself throttles once the cap is reached.
        self._max_concurrent: int = int(
            config.extra.get("max_concurrent", _DEFAULT_MAX_CONCURRENT)
        ) or _DEFAULT_MAX_CONCURRENT
        self._run_semaphore = asyncio.Semaphore(self._max_concurrent)

        # Per-route polling state: client, app_id, sink_id, consumer_id,
        # iterator. Populated by connect().
        self._route_state: Dict[str, dict] = {}
        self._poll_tasks: Dict[str, asyncio.Task] = {}
        # Poll-options class, resolved once at connect() so the poll loop need
        # not re-import the SDK on every iteration.
        self._poll_options_cls = None

        # Delivery info keyed by session chat_id.
        #
        # Read by every send() invocation for the chat_id (status messages
        # AND the final response).  Cleaned up via TTL on each event so the
        # dict stays bounded — see _prune_delivery_info().  Do NOT pop on
        # send(), or interim status messages (e.g. fallback notifications,
        # context-pressure warnings) will consume the entry before the
        # final response arrives, causing the response to silently fall
        # back to the "log" deliver type.
        self._delivery_info: Dict[str, dict] = {}
        self._delivery_info_created: Dict[str, float] = {}
        self._delivery_info_ttl: int = 3600

        # Reference to gateway runner for cross-platform delivery (set externally)
        self.gateway_runner = None

        # In-process dedup of message IDs (defense-in-depth against accidental
        # cursor non-advancement; the iterator handles the normal case).
        self._seen_message_ids: Dict[str, float] = {}

    # ------------------------------------------------------------------
    # Lifecycle
    # ------------------------------------------------------------------

    def _resolve_route_token(self, name: str, route: dict) -> str:
        """Resolve a route's auth token: literal ``auth_token`` wins over
        ``auth_token_env``. Raises ValueError when neither resolves."""
        literal = route.get("auth_token") or ""
        if literal:
            return str(literal)
        env_name = route.get("auth_token_env")
        if env_name:
            val = os.getenv(str(env_name), "")
            if not val:
                raise ValueError(
                    f"[svix] Route '{name}' sets auth_token_env={env_name!r} but "
                    f"that environment variable is empty. Export it or set a "
                    f"literal 'auth_token' on the route."
                )
            return val
        raise ValueError(
            f"[svix] Route '{name}' has no auth token. Set 'auth_token' or "
            f"'auth_token_env' on the route. Polling endpoints need an "
            f"endpoint-scoped 'sk_endp_*' token found in the Svix dashboard."
        )

    async def connect(self) -> bool:
        """Validate routes, wire per-route clients, and start polling.

        Returns ``False`` (non-retryable fatal error) on permanent
        misconfiguration — missing SDK, bad URL, unresolvable token, or
        ``deliver_only`` with no real target.
        """
        svix_api = _import_svix()
        if not svix_api:
            msg = "[svix] svix SDK not installed. Run: pip install svix"
            logger.error(msg)
            self._set_fatal_error(
                "svix_missing_dependency", msg, retryable=False
            )
            return False
        SvixAsync, SvixOptions, self._poll_options_cls = svix_api

        # Authz runs through env hooks (see register()). The host's allow-all
        # flag short-circuits before the allowlist, so an allowlist set without
        # explicitly disabling allow-all is silently ineffective — warn loudly.
        if (
            os.getenv("SVIX_ALLOW_ALL_USERS", "").strip().lower() in _TRUTHY
            and os.getenv("SVIX_ALLOWED_USERS", "").strip()
        ):
            logger.warning(
                "[svix] SVIX_ALLOWED_USERS is set but SVIX_ALLOW_ALL_USERS is "
                "truthy (the plugin defaults it to true) — allow-all wins and "
                "the allowlist is ignored. Set SVIX_ALLOW_ALL_USERS=false to "
                "enforce SVIX_ALLOWED_USERS."
            )

        if not self._routes:
            logger.warning(
                "[svix] No routes configured under platforms.svix.extra.routes — "
                "adapter will be idle."
            )

        # Validate every route up front so misconfiguration surfaces at
        # startup rather than on the first poll.
        try:
            for name, route in self._routes.items():
                # Disabled routes are kept in config (so they can be re-enabled)
                # but never polled. Default-enabled: only an explicit
                # ``enabled: false`` turns a route off, matching the upstream
                # webhook adapter (webhook.py:371).
                if route.get("enabled", True) is False:
                    logger.info("[svix] Route '%s' disabled — skipping", name)
                    continue
                url = route.get("url")
                if not url:
                    raise ValueError(
                        f"[svix] Route '{name}' has no 'url'. Each route must "
                        f"declare its Svix polling endpoint URL."
                    )
                try:
                    app_id, sink_id, server_url = _parse_polling_url(url)
                except ValueError as exc:
                    raise ValueError(f"[svix] Route '{name}': {exc}") from exc

                token = self._resolve_route_token(name, route)

                options = SvixOptions(server_url=server_url) if server_url else SvixOptions()
                self._route_state[name] = {
                    "client": SvixAsync(token, options),
                    "app_id": app_id,
                    "sink_id": sink_id,
                    # Deterministic consumer ID so the Svix server tracks our
                    # cursor and restarts resume where they left off.
                    "consumer_id": f"hermes-{name}",
                    "iterator": None,
                }

                # deliver_only routes bypass the agent — the event payload becomes a
                # direct push notification via the configured delivery target.
                # Validate up-front so misconfiguration surfaces at startup rather
                # than on the first polled event.
                if route.get("deliver_only"):
                    deliver = route.get("deliver", "log")
                    if not deliver or deliver == "log":
                        raise ValueError(
                            f"[svix] Route '{name}' has deliver_only=true but "
                            f"deliver is '{deliver}'. Direct delivery requires a "
                            f"real target (telegram, discord, slack, github_comment, etc.)."
                        )
        except ValueError as exc:
            logger.error("%s", exc)
            self._set_fatal_error("svix_invalid_config", str(exc), retryable=False)
            return False

        # Only validated, enabled routes have state — disabled/invalid ones
        # were skipped above and get no poll task.
        for name in self._route_state:
            self._poll_tasks[name] = asyncio.create_task(
                self._poll_route(name), name=f"svix-poll-{name}"
            )

        self._mark_connected()
        logger.info(
            "[svix] Polling %d route(s): %s",
            len(self._route_state),
            ", ".join(self._route_state.keys()) or "(none)",
        )
        return True

    async def disconnect(self) -> None:
        for task in self._poll_tasks.values():
            task.cancel()
        if self._poll_tasks:
            await asyncio.gather(
                *self._poll_tasks.values(), return_exceptions=True
            )
        self._poll_tasks.clear()
        self._mark_disconnected()
        logger.info("[svix] Disconnected")

    # ------------------------------------------------------------------
    # Polling loop
    # ------------------------------------------------------------------

    async def _poll_route(self, route_name: str) -> None:
        """Long-running poll loop for a single route.

        Polls the server-tracked consumer iterator (kept in memory; on restart
        the first call omits it so the server resumes from the consumer's
        position). ``done == True`` means caught up — wait ``poll_interval``.

        Delivery is at-most-once w.r.t. agent completion: the cursor advances
        when a page is processed, not when the dispatched agent run finishes,
        so a crash mid-run drops that in-flight event. Fine for one-shot
        webhook reviews; don't rely on it for work that must not be lost.
        """
        state = self._route_state[route_name]
        client = state["client"]
        MessagePollerConsumerPollOptions = self._poll_options_cls
        backoff = 1.0
        max_backoff = 60.0

        while True:
            try:
                options = MessagePollerConsumerPollOptions(
                    limit=self._poll_limit,
                    iterator=state["iterator"],
                )
                result = await client.message.poller.consumer_poll(
                    state["app_id"], state["sink_id"], state["consumer_id"],
                    options,
                )
                backoff = 1.0

                # Prune the dedup set once per page (not once per message).
                now = time.time()
                self._seen_message_ids = {
                    k: t for k, t in self._seen_message_ids.items()
                    if now - t < _DEDUP_TTL
                }

                fresh = 0
                for message in result.data:
                    try:
                        if await self._process_message(route_name, message):
                            fresh += 1
                    except Exception:
                        logger.exception(
                            "[svix] Error processing message %s on route %s",
                            getattr(message, "id", "?"),
                            route_name,
                        )

                advanced = bool(result.iterator)
                if advanced:
                    state["iterator"] = result.iterator

                if result.done:
                    await asyncio.sleep(self._poll_interval)
                elif not result.data:
                    await asyncio.sleep(_EMPTY_PAGE_SLEEP)
                elif fresh == 0 and not advanced:
                    # Non-empty page, every message a dedup re-seen, AND the
                    # iterator didn't advance — re-polling returns the same page
                    # in a tight loop bounded only by HTTP RTT. Pace it and
                    # surface the stall (the dedup set was built for exactly this).
                    logger.warning(
                        "[svix] Route %s: page of %d message(s) yielded no new "
                        "messages and the cursor did not advance — pacing to "
                        "avoid a hot poll loop.",
                        route_name,
                        len(result.data),
                    )
                    await asyncio.sleep(self._poll_interval)
                # else: progress (new messages or the cursor advanced) — loop
                # immediately to drain the backlog
            except asyncio.CancelledError:
                raise
            except Exception as exc:
                logger.warning(
                    "[svix] Poll error on route %s: %s (retrying in %.1fs)",
                    route_name,
                    exc,
                    backoff,
                )
                try:
                    await asyncio.sleep(backoff)
                except asyncio.CancelledError:
                    raise
                backoff = min(backoff * 2, max_backoff)

    # ------------------------------------------------------------------
    # Event processing
    # ------------------------------------------------------------------

    async def _process_message(self, route_name: str, message: Any) -> bool:
        """Dedup, filter by event type, render the prompt, then either
        direct-deliver or dispatch the agent via ``handle_message()``.

        Returns ``True`` when the message was newly seen (the cursor advanced
        for it), ``False`` when it was dedup-skipped — the poll loop uses this
        to detect a non-advancing cursor and pace itself.
        """
        route_config = self._routes.get(route_name)
        if not route_config:
            return False

        message_id = str(getattr(message, "id", ""))
        event_type = str(getattr(message, "event_type", "") or "unknown")
        payload = getattr(message, "payload", None) or {}

        now = time.time()
        if message_id and message_id in self._seen_message_ids:
            logger.debug(
                "[svix] Skipping already-seen message %s on route %s",
                message_id,
                route_name,
            )
            return False
        if message_id:
            self._seen_message_ids[message_id] = now

        # Check event type filter
        allowed_events = route_config.get("events") or []
        if allowed_events and event_type not in allowed_events:
            logger.debug(
                "[svix] Ignoring event %s for route %s (allowed: %s)",
                event_type,
                route_name,
                allowed_events,
            )
            # Newly seen (recorded above); the cursor should advance past it.
            return True

        prompt = self._render_prompt(
            route_config.get("prompt", ""), payload, event_type, route_name
        )
        prompt = self._inject_skill(prompt, route_config.get("skills", []))

        delivery_id = message_id or str(int(time.time() * 1000))
        deliver_extra = self._render_delivery_extra(
            route_config.get("deliver_extra", {}), payload, event_type
        )

        # ── Direct delivery mode (deliver_only) ─────────────────
        # Skip the agent entirely — the rendered prompt IS the message we
        # deliver.  Use case: external services (Supabase, monitoring,
        # cron jobs, other agents) that need to push a plain notification
        # to a user's chat with zero LLM cost.  Reuses the same dedup,
        # event filtering, and template rendering as agent mode.
        if route_config.get("deliver_only"):
            delivery = {
                "deliver": route_config.get("deliver", "log"),
                "deliver_extra": deliver_extra,
                "payload": payload,
            }
            logger.info(
                "[svix] direct-deliver event=%s route=%s target=%s msg_id=%s",
                event_type,
                route_name,
                delivery["deliver"],
                delivery_id,
            )
            # Delivery is at-most-once; failures can't be retried (the cursor
            # already advanced), so a dropped notification must at least be
            # visible. Upstream logs a warning and returns 502 here.
            result = await self._direct_deliver(prompt, delivery)
            if not result.success:
                logger.warning(
                    "[svix] direct-deliver failed event=%s route=%s target=%s "
                    "msg_id=%s error=%s",
                    event_type,
                    route_name,
                    delivery["deliver"],
                    delivery_id,
                    result.error,
                )
            return True

        # Use delivery_id in session key so concurrent webhooks on the
        # same route get independent agent runs (not queued/interrupted).
        session_chat_id = f"svix:{route_name}:{delivery_id}"

        # Store delivery info for send().  Read by every send() invocation
        # for this chat_id (interim status messages and the final response),
        # so we do NOT pop on send.  TTL-based cleanup keeps the dict bounded.
        self._delivery_info[session_chat_id] = {
            "deliver": route_config.get("deliver", "log"),
            "deliver_extra": deliver_extra,
            "payload": payload,
        }
        self._delivery_info_created[session_chat_id] = now
        self._prune_delivery_info(now)

        # Build source and event
        source = self.build_source(
            chat_id=session_chat_id,
            chat_name=f"svix/{route_name}",
            chat_type="webhook",
            user_id=f"svix:{route_name}",
            user_name=route_name,
        )
        event = MessageEvent(
            text=prompt,
            message_type=MessageType.TEXT,
            source=source,
            raw_message=payload,
            message_id=delivery_id,
        )

        logger.info(
            "[svix] event=%s route=%s prompt_len=%d msg_id=%s",
            event_type,
            route_name,
            len(prompt),
            delivery_id,
        )

        # Acquire a concurrency slot before dispatching. This awaits when the
        # cap is reached, so the poll loop (which awaits us) stops fetching new
        # pages until an in-flight agent run finishes — bounded backpressure on
        # a backlog drain rather than unbounded fan-out.
        await self._run_semaphore.acquire()
        task = asyncio.create_task(self._run_agent(event))
        self._background_tasks.add(task)
        task.add_done_callback(self._background_tasks.discard)
        return True

    async def _run_agent(self, event: MessageEvent) -> None:
        """Run one agent dispatch and release its concurrency slot."""
        try:
            await self.handle_message(event)
        finally:
            self._run_semaphore.release()

    async def get_chat_info(self, chat_id: str) -> Dict[str, Any]:
        return {"name": chat_id, "type": "svix"}


# ----------------------------------------------------------------------
# Registry glue
# ----------------------------------------------------------------------

def _is_connected(config: PlatformConfig) -> bool:
    """True when the adapter should run: enabled (via config or ``SVIX_ENABLED``)
    with at least one route configured.

    NOTE: we intentionally do NOT special-case an explicit
    ``platforms.svix.enabled: false`` here. The host calls this from three
    places with different ``config`` shapes — the runtime gate
    (``get_connected_platforms`` → ``_is_platform_connected``, real config,
    only when ``enabled`` is already True) and two probes that force
    ``enabled=True`` (``config.py`` enable pass, ``hermes_cli`` status). The
    ``_enabled_explicit`` marker can be present in any of them, so keying off it
    here also disables a genuine ``enabled: true`` and stops the platform from
    loading at all. Honoring an explicit disable for a plugin platform is a
    host-side concern (``_enable_from_env``); the plugin can't do it from
    ``is_connected`` without that false-positive.
    """
    enabled = bool(getattr(config, "enabled", False)) or (
        os.getenv("SVIX_ENABLED", "").strip().lower() in _TRUTHY
    )
    return enabled and bool(config.extra.get("routes"))


def _env_enablement() -> Optional[dict]:
    """Seed poll-tuning knobs from env vars at config load. Returns ``None``
    unless ``SVIX_ENABLED`` is truthy. Routes still come from config.yaml."""
    if os.getenv("SVIX_ENABLED", "").strip().lower() not in _TRUTHY:
        return None
    seed: dict = {}
    poll_interval = os.getenv("SVIX_POLL_INTERVAL", "").strip()
    if poll_interval:
        try:
            seed["poll_interval"] = float(poll_interval)
        except ValueError:
            pass
    poll_limit = os.getenv("SVIX_POLL_LIMIT", "").strip()
    if poll_limit:
        try:
            seed["poll_limit"] = int(poll_limit)
        except ValueError:
            pass
    return seed


def register(ctx):
    """Plugin entry point: called by the Hermes plugin system."""
    # Polled events are authenticated by the endpoint-scoped token, not a human
    # user — default to allow-all so the allowlist doesn't reject the synthetic
    # ``svix:<route>`` users. ``setdefault`` preserves an explicit override.
    # NOTE: the host checks allow-all before the allowlist, so an operator who
    # wants SVIX_ALLOWED_USERS enforced must set SVIX_ALLOW_ALL_USERS=false
    # (connect() warns when both are set). See README "Authorization".
    os.environ.setdefault("SVIX_ALLOW_ALL_USERS", "true")

    # The host posts a one-time "no home channel set — /sethome" notice to the
    # source on the first turn for any platform except local/webhook. Svix turns
    # have no interactive user and deliver to configured targets, so that nag
    # would be posted to every route's deliver target (a PR comment, a Telegram
    # message) before the real response. Default the home-target env var so the
    # host's ``if not os.getenv(env_key)`` check suppresses it; ``setdefault``
    # preserves a real home channel if the operator sets one.
    os.environ.setdefault("SVIX_HOME_CHANNEL", "-")

    # Svix delivers one-shot webhook reviews, so default to delivering only the
    # final response (mid-turn chatter would become extra comments/messages).
    # This seeds the lowest-precedence display tier, so a user global or
    # HERMES_TOOL_PROGRESS_MODE still wins. There's no registry hook for display
    # defaults today, so this is best-effort — log if it can't seed rather than
    # silently reverting svix to the chatty global defaults.
    try:
        from gateway.display_config import _PLATFORM_DEFAULTS
        _PLATFORM_DEFAULTS.setdefault("svix", {}).update({
            "tool_progress": "off",
            "interim_assistant_messages": False,
        })
    except Exception as exc:  # pragma: no cover - display internals may move
        logger.warning(
            "[svix] Could not seed display defaults (%s); tool-progress lines "
            "and interim messages may be delivered as separate messages. Set "
            "display.platforms.svix in config.yaml to suppress them.",
            exc,
        )

    ctx.register_platform(
        name="svix",
        label="Svix",
        adapter_factory=lambda cfg: SvixAdapter(cfg),
        check_fn=check_svix_requirements,
        is_connected=_is_connected,
        required_env=[],  # per-route tokens live in config.yaml / route env vars
        install_hint="pip install svix",
        env_enablement_fn=_env_enablement,
        # Authorization runs through these env hooks (defaulted to allow-all
        # above) because core can't auto-exempt a plugin platform.
        allow_all_env="SVIX_ALLOW_ALL_USERS",
        allowed_users_env="SVIX_ALLOWED_USERS",
        emoji="🪝",
        # Inbound-only; responses fan out via `deliver`, so no length cap.
        max_message_length=0,
        pii_safe=False,
        platform_hint=_SVIX_PLATFORM_HINT,
    )
