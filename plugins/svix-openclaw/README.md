# Svix OpenClaw

An OpenClaw plugin that receives webhook events by **polling Svix sinks** with
the SDK's `AutoConfigConsumer` instead of exposing an inbound HTTP server. Each
polled message is delivered to OpenClaw one of two ways — only the transport
(pull instead of push) changes:

- **TaskFlow actions** — the payload is applied as an upstream `webhooks`
  extension action (`create_flow`, `run_task`, …) against a bound TaskFlow
  session.
- **Gateway hooks** — the payload is `POST`ed to OpenClaw's documented
  [automation webhooks](https://docs.openclaw.ai/automation/cron-jobs#webhooks),
  `/hooks/wake` (enqueue a system event for the main session) or `/hooks/agent`
  (run an isolated agent turn).

TaskFlow delivery is configured per `route`; the `wake` and `agent` gateway hooks
are configured once at the top level (there is a single gateway). The three are
independent Svix polling sinks, so you point one poller at a `wake` sink, another
at an `agent` sink, and one or more routes at TaskFlow sinks. Each poller is
configured with a single Svix **AutoConfig token** (`auto_v1_…`) — it embeds the
application id, sink id, server URL, and API token, and the plugin uses it both
to provision the sink (`subscribe()`) and to drain it (`receive()`/`commit()`).

## Why

OpenClaw's webhook surfaces are **push**-based: the bundled `webhooks` extension
(`extensions/webhooks/`) registers an inbound HTTP route for TaskFlow actions,
and the gateway exposes `/hooks/wake` + `/hooks/agent` for automation. Both
require the host to be reachable — a public URL, an open port, or a tunnel.

Many deployments can't (or won't) expose an inbound server: agents behind NAT,
locked-down networks, or setups that already have a durable message buffer in
front of them. This plugin inverts the direction. Nothing listens; background
pollers drain Svix sinks with the official
[`svix`](https://www.npmjs.com/package/svix) SDK's
[`AutoConfigConsumer`](https://docs.svix.com/receiving/webhooks-autoconfig) — you
give each poller a single **AutoConfig token** (`auto_v1_…`) — and deliver each
buffered message's payload to OpenClaw exactly as an inbound `POST` would. The
consumer is offset/lease based: it provisions the sink with `subscribe()`, then
leases a batch with `receive()` and acks it with `commit()`, with the read cursor
tracked server-side under a deterministic consumer id so restarts resume cleanly.

## How it maps onto OpenClaw's webhook systems

```
                 OpenClaw (push)                          this plugin (pull)
                 ───────────────────────────────          ──────────────────────────────
  transport      inbound HTTP route                        AutoConfigConsumer poll loop
  auth           presented shared secret / hooks.token     AutoConfig token (auto_v1_…)
  ─────────────────────────────────────────────────────────────────────────────────────
  TaskFlow       POST <webhooks route>                      poll route → executeWebhookAction
                 webhookActionSchema → execute → classify   (validate → apply → classify)
  ─────────────────────────────────────────────────────────────────────────────────────
  wake           POST /hooks/wake   { text, mode }          poll `wake` sink  → POST /hooks/wake
  agent          POST /hooks/agent  { message, … }          poll `agent` sink → POST /hooks/agent
```

The TaskFlow core is the part worth not re-implementing. It lives in
[`src/vendor/webhook-actions.ts`](src/vendor/webhook-actions.ts), copied
**verbatim** from upstream `extensions/webhooks/src/http.ts` (minus the HTTP-only
functions). The thin abstraction in
[`src/processor.ts`](src/processor.ts) — `processWebhookAction()` — exposes that
`validate → execute → classify` pipeline transport-free.

For the `wake`/`agent` pollers there is nothing to vendor: the plugin simply
`POST`s the polled payload to the local gateway hooks endpoint (`/hooks/wake` or
`/hooks/agent`), authenticating with the `hooks.token` from your OpenClaw config.
The polled payload **is** the hook request body, so it must match the documented
shape (`{ text, mode }` for wake; `{ message, name?, … }` for agent).

## Files

| File | Role |
| --- | --- |
| `index.ts` | Plugin entry. Builds a dispatcher per poller — TaskFlow (`bindSession` + `processWebhookAction`) or hook (`POST` to `/hooks/wake` / `/hooks/agent`) — and registers a background service that runs them. |
| `src/config.ts` | Poller config schema + `resolveWebhookPollerConfig` (resolves each route into a TaskFlow poller, plus the top-level `wake` / `agent` pollers). Also defines `WebhookSecretInput`. |
| `src/poller.ts` | The polling transport: `AutoConfigConsumer` loop — `subscribe()` to provision the sink, then `receive()` → hands each message to a `dispatch` callback → `commit()` to ack the batch and advance the server-side cursor. |
| `src/processor.ts` | Transport-agnostic abstraction over the vendored TaskFlow core (`validate → execute → classify`). |
| `src/vendor/webhook-actions.ts` | **Vendored from OpenClaw.** Action schemas, TaskFlow execution, and result mapping, copied verbatim. |
| `api.ts`, `runtime-api.ts` | Re-export shims for the `openclaw/plugin-sdk/*` SDK, mirroring the upstream extension. |

## Setup

There are two sides to wire up: one or more **Svix Ingest sources + sinks** (the
buffers messages land in), and the **OpenClaw plugin** (the pollers that drain
them).

Create **one sink per destination** you want to feed:

- a TaskFlow sink → route `token`
- a wake sink → top-level `wake.token` → `POST /hooks/wake`
- an agent sink → top-level `agent.token` → `POST /hooks/agent`

The `wake`/`agent` destinations also require OpenClaw's gateway hooks to be
enabled (`hooks.enabled: true` + a `hooks.token`) — see step 3.

### 1. Create a Svix Ingest source and grab an AutoConfig token

For each destination you need **one value** — a Svix
[AutoConfig](https://docs.svix.com/receiving/webhooks-autoconfig) token
(`auto_v1_…`). It encodes the application id, sink id, server URL, and API token,
so it is all the plugin needs: it provisions the polling sink for you on startup
(`subscribe()`) and then drains it. Repeat once per destination (TaskFlow / wake
/ agent).

1. In the [Svix dashboard](https://dashboard.svix.com) go to **Svix Ingest →
   Sources → Create source**. Name it (e.g. `openclaw`).
2. Pick the **Source Type**. Use **Generic Webhook** when your own automation
   produces the payloads; pick a provider (GitHub, Stripe, …) to have Svix verify
   that provider's signatures (then enable authentication and store the secret).
3. Copy the source's **Ingest URL** — the public URL events are `POST`ed to. Hand
   it to whatever produces the events (your automation, a provider webhook, etc.).
4. Generate an **AutoConfig token** for the source's destination and copy the
   `auto_v1_…` value. That token goes straight into `token` below; the plugin's
   `subscribe()` call provisions the matching polling sink the first time it runs
   (no need to create the Polling Endpoint by hand).

The plugin sets the sink's `filterTypes`/`channels` from your config when it
provisions the sink, so you can narrow what each destination buffers from
OpenClaw rather than in the portal.

### 2. Install the plugin into OpenClaw

Link this plugin directory into your OpenClaw install (adds the load path and a
`plugins.entries.svix-openclaw` entry):

```bash
openclaw plugins install --link /path/to/svix/ai/plugins/svix-openclaw
```

### 3. Configure routes (and enable hooks for wake/agent)

Add TaskFlow routes under `plugins.entries.svix-openclaw.config.routes`, and the
`wake`/`agent` pollers alongside them at `plugins.entries.svix-openclaw.config`
(see the field reference in [Configuration](#configuration) below), using the
AutoConfig token(s) from step 1. Store tokens via env secret refs rather than
inline.

If you configure `wake`/`agent`, enable OpenClaw's gateway hooks so the plugin
can `POST` to them (the plugin reads `hooks.token` and the gateway port from your
config — no extra plugin fields needed):

```jsonc
{
  "hooks": { "enabled": true, "token": "a-strong-shared-secret", "path": "/hooks" }
}
```

### 4. Run and verify

```bash
openclaw gateway --verbose
```

Look for a log line per poller, e.g.
`[svix-openclaw] polling Svix consumer=svix-openclaw/agent -> agent (poller agent)`
(and, when `subscribe` is enabled, `provisioned polling sink`). Then send a test
payload to the Ingest URL whose sink feeds that poller:

```bash
# TaskFlow sink
curl -X POST "$TASKFLOW_INGEST_URL" -H 'Content-Type: application/json' \
  -d '{ "action": "create_flow", "goal": "Investigate alert" }'

# wake sink  -> /hooks/wake
curl -X POST "$WAKE_INGEST_URL" -H 'Content-Type: application/json' \
  -d '{ "text": "New email received", "mode": "now" }'

# agent sink -> /hooks/agent
curl -X POST "$AGENT_INGEST_URL" -H 'Content-Type: application/json' \
  -d '{ "message": "Summarize inbox", "name": "Email" }'
```

Each payload lands in its sink's buffer; the matching poller leases it on its next
`receive()`, dispatches it (applies the TaskFlow action or POSTs it to the gateway
hook), then `commit()`s the batch. A successful dispatch logs `dispatched … -> 2xx`.

## Configuration

Configured under `plugins.entries.svix-openclaw.config` in your OpenClaw config.
Each entry in `routes` is a TaskFlow poller (`token` + `sessionKey`). The `wake`
and `agent` pollers sit alongside `routes`, one of each. Configure at least one
poller (a route, `wake`, or `agent`) — with none, the plugin loads but starts no
pollers and does nothing.

```jsonc
{
  "plugins": {
    "entries": {
      "svix-openclaw": {
        "enabled": true,
        "config": {
          // TaskFlow pollers: each route's payloads are applied as TaskFlow actions.
          "routes": {
            "ops": {
              // Svix AutoConfig token (auto_v1_…) — embeds app id, sink id, server URL, API token.
              "token": { "source": "env", "provider": "env", "id": "SVIX_TASKFLOW_TOKEN" },
              "sessionKey": "agent:main",
              "controllerId": "svix-openclaw/ops"
            }
          },

          // wake poller (optional): payloads POSTed to /hooks/wake.
          "wake": {
            "token": { "source": "env", "provider": "env", "id": "SVIX_WAKE_TOKEN" }
          },

          // agent poller (optional): payloads POSTed to /hooks/agent.
          "agent": {
            "token": { "source": "env", "provider": "env", "id": "SVIX_AGENT_TOKEN" },
            "filterTypes": ["email.received"],
            "pollIntervalMs": 5000,
            "limit": 50
          }
        }
      }
    }
  }
}
```

**Top-level fields**

| Field | Required | Default | Meaning |
| --- | --- | --- | --- |
| `routes` | — | `{}` | Map of route id → TaskFlow poller (see [Route fields](#route-fields)). |
| `wake` | — | — | A [hook endpoint](#hook-endpoint-fields) whose messages are POSTed to `/hooks/wake`. |
| `agent` | — | — | A [hook endpoint](#hook-endpoint-fields) whose messages are POSTed to `/hooks/agent`. |

Configure at least one of a route, `wake`, or `agent` — otherwise the plugin
starts no pollers.

<a name="route-fields"></a>**Route fields** (each entry in `routes`)

| Field | Required | Default | Meaning |
| --- | --- | --- | --- |
| `token` | ✅ | — | Svix AutoConfig token (`auto_v1_…`) for the TaskFlow sink. Inline string or `{ source, provider, id }` secret ref. |
| `sessionKey` | ✅ | — | TaskFlow session the actions are applied to. |
| `controllerId` | | `svix-openclaw/<routeId>` | Controller id stamped on managed flows. |
| `enabled` | | `true` | Set `false` to skip the whole route. |

The sink-provisioning + poll-tuning fields below (`subscribe`, `filterTypes`,
`channels`, `consumerId`, `startingPosition`, `leaseDurationMs`, `pollIntervalMs`,
`limit`, `payloadField`) also apply at the route level.

<a name="hook-endpoint-fields"></a>**Hook endpoint fields** (`wake` / `agent`)

| Field | Required | Default | Meaning |
| --- | --- | --- | --- |
| `token` | ✅ | — | Svix AutoConfig token (`auto_v1_…`) for this hook's sink. Inline string or a secret ref. |
| `subscribe` | | `true` | Provision (create/update) the sink on startup via `subscribe()`. Set `false` if the sink is managed elsewhere. |
| `filterTypes` | | — | Event types the sink buffers (applied at `subscribe()` time). Omit for all. |
| `channels` | | — | Channels the sink listens to (applied at `subscribe()` time). Omit for all. |
| `consumerId` | | `svix-openclaw.<id>` | Deterministic consumer id the server tracks the read offset under. Svix consumer group names allow only alphanumerics, `_`, `-`, and `.`. |
| `startingPosition` | | `latest` | Where a brand-new consumer starts (`earliest`\|`latest`). Only honored on its first poll. |
| `leaseDurationMs` | | server default | Lease duration for a polled batch before it can be re-leased. |
| `pollIntervalMs` | | `5000` | Idle wait after the sink reports it is drained (`done: true`). |
| `limit` | | `50` | Page size per poll. |
| `payloadField` | | `payload` | Field on each Svix message holding the body. Empty string ⇒ the whole message. |

> Auth + base URL for the hook `POST`s are read from your OpenClaw `hooks.token`
> and `gateway.port` — set `hooks.enabled: true` and a `hooks.token` (see
> [Setup step 3](#3-configure-routes-and-enable-hooks-for-wakeagent)).

### Message payloads

Each poller leases a batch with `receive(consumerId, { limit, … })`, walks the
returned `PollerV2PollOut` (`data[]`, `done`), `commit()`s the highest offset it
processed to ack the batch and advance the server-side cursor, and idles for
`pollIntervalMs` once `done` is `true`.

Each Svix message's `payload` is used verbatim as the request body for that
poller's destination, so the payload shape depends on which poller buffered it.

**TaskFlow (route)** — a webhook action object, the same JSON the upstream HTTP
route accepts:

```jsonc
{ "action": "create_flow", "goal": "Investigate alert" }
```

See upstream `extensions/webhooks` for the full action catalog (`create_flow`,
`get_flow`, `list_flows`, `find_latest_flow`, `resolve_flow`, `get_task_summary`,
`set_waiting`, `resume_flow`, `finish_flow`, `fail_flow`, `request_cancel`,
`cancel_flow`, `run_task`).

**`wake`** — a `/hooks/wake` body (enqueue a system event for the main session):

```jsonc
{ "text": "New email received", "mode": "now" }   // mode: "now" | "next-heartbeat"
```

**`agent`** — a `/hooks/agent` body (run an isolated agent turn):

```jsonc
{ "message": "Summarize inbox", "name": "Email", "model": "anthropic/claude-sonnet-4-6" }
```

The `/hooks/agent` body also accepts `agentId`, `wakeMode`, `deliver`, `channel`,
`to`, `fallbacks`, `thinking`, and `timeoutSeconds`. See the OpenClaw
[automation webhooks docs](https://docs.openclaw.ai/automation/cron-jobs#webhooks)
for the authoritative field list. If your producer can't emit these shapes
directly, use a Svix transformation on the Polling Endpoint (or a `hooks.mappings`
entry on the OpenClaw side) to reshape the payload.

## Keeping the vendored core in sync with OpenClaw

`src/vendor/webhook-actions.ts` is a deliberate copy so upstream changes are easy
to fold in. To update:

1. Copy upstream `extensions/webhooks/src/http.ts`.
2. Drop the HTTP-transport-only functions (`writeJson`, `extractSharedSecret`,
   `timingSafeEquals`, `createTaskFlowWebhookRequestHandler`).
3. Re-apply the two documented deltas (the three import lines, and the `export`
   keyword on the symbols the poller consumes).

The abstraction (`processor.ts`) and the transport (`poller.ts`) depend only on
those exported symbols, so they should not need changes when re-syncing.
