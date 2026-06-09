# Svix platform plugin for Hermes Agent

A [Hermes Agent](https://github.com/NousResearch/hermes-agent) gateway plugin
that lets Hermes **receive webhooks through [Svix](https://www.svix.com/)** without
hosting a public HTTP server.

Svix receives the upstream webhook, this adapter **polls** Svix's
[polling endpoint API](https://docs.svix.com/advanced-endpoints/polling-endpoints)
for new messages, and each event flows through a route → prompt → delivery
pipeline (the same shape as Hermes' built-in `webhook` adapter). Because it
polls, it works from laptops behind NAT, dev boxes, and restricted networks
where you can't expose an ingress.

## Install

Drop this directory into one of Hermes' plugin locations:

```bash
# user-level
cp -r plugins/svix-hermes ~/.hermes/plugins/svix-hermes
```

Install the Svix SDK:

```bash
pip install svix
```

Enable the plugin:

```bash
hermes plugins enable svix-hermes   # not needed for bundled platform plugins
```

## Configure

Add routes under `platforms.svix.extra.routes` in your gateway `config.yaml`:

```yaml
platforms:
  svix:
    enabled: true
    extra:
      poll_interval: 5    # seconds to wait after Svix reports caught-up
      poll_limit: 50      # max messages fetched per poll request
      max_concurrent: 5   # cap on concurrent agent runs (backlog backpressure)
      routes:
        github_events:
          url: https://api.svix.com/api/v1/app/app_xxx/poller/poll_yyy/
          # Per-route auth. Polling endpoints use an endpoint-scoped
          # token (sk_endp_*); pass it inline or via an env var.
          auth_token_env: SVIX_GITHUB_INGEST_TOKEN
          prompt: "GitHub issue opened: {issue.title}\n\n{__raw__}"
          deliver: telegram                     # log | github_comment | any connected platform
          deliver_extra:
            chat_id: "-1001234567890"
```

Or enable from the environment (routes still come from `config.yaml`):

```bash
export SVIX_ENABLED=true
export SVIX_POLL_INTERVAL=5
export SVIX_POLL_LIMIT=50
```

### Getting the polling URL and token

The [Svix CLI](https://docs.svix.com/cli) is the quickest path:

```bash
brew install svix/svix/svix-cli
svix login
svix ingest source list              # find your source
svix ingest source get <source_id>   # shows the polling URL + token
```

### Route fields

| Field | Description |
| --- | --- |
| `url` | Svix polling endpoint, `https://<host>/api/v1/app/<app_id>/poller/<sink_id>/`. Region/self-hosted hosts (e.g. `api.eu.svix.com`) work automatically. |
| `auth_token` / `auth_token_env` | Endpoint-scoped `sk_endp_*` token, inline or from an env var. Literal wins over env. |
| `events` | Optional allowlist of `eventType`s; others are ignored. |
| `prompt` | Template rendered against the payload. See [Templates](#prompt-templates). |
| `skills` | Skills to invoke; the first loadable one wraps the prompt. |
| `deliver` | Where the agent's response goes: `log`, `github_comment`, or any connected platform (`telegram`, `discord`, `slack`, …). |
| `deliver_extra` | Target details (`chat_id`, `repo`, `pr_number`, `thread_id`…); string values are payload-templated. |
| `deliver_only` | If `true`, deliver the rendered prompt directly without running the agent (requires a real `deliver` target, not `log`). |
| `enabled` | Default `true`; set `false` to keep the route in config but stop polling it (mirrors the built-in `webhook` adapter). |

### Prompt templates

- `{a.b.c}` — dot-notation into the payload (`{pull_request.title}`). Missing
  keys are left verbatim so typos are visible.
- `{__raw__}` — the full payload as indented JSON (capped at 4000 chars).
- `{__event__}` — the event type.
- Empty `prompt` → a JSON dump of the payload with event/route context.

## Authorization

Polled events carry no human user — they're authenticated by the endpoint-scoped
token on each route — so each event runs under a synthetic `svix:<route>`
identity. To keep the host's allowlist from rejecting those, the plugin defaults
`SVIX_ALLOW_ALL_USERS=true` at registration.

| Env var | Effect |
| --- | --- |
| `SVIX_ALLOW_ALL_USERS` | Defaulted to `true`. When truthy, every route is authorized. |
| `SVIX_ALLOWED_USERS` | Comma-separated allowlist of `svix:<route>` identities. |

The host checks allow-all **before** the allowlist, so `SVIX_ALLOWED_USERS` has
no effect unless you also set `SVIX_ALLOW_ALL_USERS=false`. The adapter logs a
warning at startup if both are set. To restrict which routes may run agents:

```bash
export SVIX_ALLOW_ALL_USERS=false
export SVIX_ALLOWED_USERS=svix:github_events,svix:billing
```

## How it works

- **Cursor tracking.** Each route polls with a stable consumer ID
  (`hermes-<route>`); Svix tracks that consumer's position server-side, so an
  interrupted process never re-fetches already-seen pages. The in-memory
  iterator is never persisted — on restart the first poll omits it and the
  server resumes from the tracked position.
- **Delivery semantics.** At-most-once with respect to agent completion: a
  message's agent run is dispatched as a background task and the cursor
  advances when the page is processed, not when the agent finishes — so a
  crash mid-run drops that in-flight event. This suits one-shot webhook
  reviews; don't rely on it for work that must not be lost.
- **Delivery.** Responses route to `log`, a GitHub PR/issue comment (via the
  `gh` CLI), or any other connected gateway platform (cross-platform delivery
  is wired automatically — the gateway injects its runner into the adapter).
- **Dedup.** In-process message-ID dedup backs up the iterator as
  defense-in-depth.


## Files

- `plugin.yaml` — manifest (`kind: platform`).
- `adapter.py` — `SvixAdapter` + `register()` entry point.
- `delivery.py` — bundled render → deliver pipeline (`WebhookDeliveryMixin`).
- `__init__.py` — exports `register`.

## License

MIT
