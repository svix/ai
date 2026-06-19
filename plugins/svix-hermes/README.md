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

Polling is driven by the Svix SDK's **`AutoConfigConsumer`**: each route holds
a single `auto_v1_*` [AutoConfig](https://docs.svix.com/receiving/webhooks-autoconfig)
token (which embeds the app id, sink id, and server URL — no URL to configure).
On startup the plugin calls `subscribe()` to provision the polling endpoint
from the route's event filters, then loops `receive()` → dispatch →
`commit(offset)` to drain messages with an explicitly committed cursor.

## Install

Drop this directory into one of Hermes' plugin locations:

```bash
# user-level
cp -r plugins/svix-hermes ~/.hermes/plugins/svix-hermes
```

Install the Svix SDK (AutoConfig consumer support landed in 1.96.0):

```bash
pip install 'svix>=1.96.0'
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
          # AutoConfig token (auto_v1_*) from the Svix dashboard. It embeds
          # the app id, sink id, and server URL — no URL to configure. Pass
          # it inline (`token:`) or via an env var (`token_env:`).
          token_env: SVIX_GITHUB_AUTOCONFIG_TOKEN
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

### Getting the AutoConfig token

Create an AutoConfig polling endpoint in the [Svix dashboard](https://dashboard.svix.com)
(**Endpoints → Add Endpoint → AutoConfig**) and copy the `auto_v1_*` token it
shows once. The token embeds the app id, sink id, and server URL — region /
self-hosted deployments (e.g. `api.eu.svix.com`) work automatically with no
extra config. Store it in an env var and reference it with `token_env`.

Rotating the token in the dashboard invalidates the old one; update `token` /
`token_env` and restart.

### Route fields

| Field | Description |
| --- | --- |
| `token` / `token_env` | AutoConfig token (`auto_v1_*`), inline or from an env var. Literal wins over env. |
| `channels` | Optional list of Svix channels to subscribe the endpoint to (`subscribe()` only). |
| `prompt` | Template rendered against the payload. See [Templates](#prompt-templates). |
| `skills` | Skills to invoke; the first loadable one wraps the prompt. |
| `deliver` | Where the agent's response goes: `log`, `github_comment`, or any connected platform (`telegram`, `discord`, `slack`, …). |
| `deliver_extra` | Target details (`chat_id`, `repo`, `pr_number`, `thread_id`…); string values are payload-templated. |
| `deliver_only` | If `true`, deliver the rendered prompt directly without running the agent (requires a real `deliver` target, not `log`). |
| `enabled` | Default `true`; set `false` to keep the route in config but stop polling it (mirrors the built-in `webhook` adapter). |

Two optional poll-tuning knobs live alongside `poll_interval` / `poll_limit` /
`max_concurrent` under `platforms.svix.extra`:

| Field | Description |
| --- | --- |
| `lease_duration_ms` | How long a received batch stays leased before Svix may re-deliver it. Omit for the server default (~5 min). Each batch is committed right after dispatch, so this only matters if the process stalls mid-batch. |
| `starting_position` | `latest` (default) skips any backlog on a consumer's **first** poll; `earliest` replays it. Ignored once an offset has been committed. |

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

- **Subscribe.** On the first poll cycle each route calls the consumer's
  `subscribe()`, which idempotently creates/updates the polling endpoint from
  the route's `events`/`channels`. It's best-effort: if it fails the route
  still polls (the endpoint may already be configured), logging one warning.
- **Cursor tracking.** Each route polls with a stable consumer ID
  (`hermes-<route>`); Svix tracks that consumer's committed offset
  server-side, so an interrupted process resumes where it left off. After a
  batch is dispatched the plugin `commit()`s the batch's highest offset,
  advancing the cursor and releasing the server-side lease.
- **Delivery semantics.** At-least-once with respect to dispatch: the offset
  is committed only after every message in the batch has been dispatched, so a
  crash before commit re-delivers the batch (the in-process dedup set absorbs
  duplicates within its TTL; the server's lease also prevents another poll from
  re-reading uncommitted messages). It remains at-most-once with respect to
  agent completion — commit happens at dispatch time, not when the agent run
  finishes — so a crash mid-run still drops that in-flight event. This suits
  one-shot webhook reviews; don't rely on it for work that must not be lost.
- **Delivery.** Responses route to `log`, a GitHub PR/issue comment (via the
  `gh` CLI), or any other connected gateway platform (cross-platform delivery
  is wired automatically — the gateway injects its runner into the adapter).
- **Dedup.** In-process message-ID dedup backs up the committed offset as
  defense-in-depth across the crash-before-commit redelivery window.


## Files

- `plugin.yaml` — manifest (`kind: platform`).
- `adapter.py` — `SvixAdapter` + `register()` entry point.
- `delivery.py` — bundled render → deliver pipeline (`WebhookDeliveryMixin`).
- `__init__.py` — exports `register`.

## License

MIT
