# Svix OpenClaw

An OpenClaw plugin that receives webhook events by **polling Svix Polling
Endpoints** instead of exposing an inbound HTTP server. Each polled message is
delivered to OpenClaw one of two ways — only the transport (pull instead of
push) changes:

- **TaskFlow actions** — the payload is applied as an upstream `webhooks`
  extension action (`create_flow`, `run_task`, …) against a bound TaskFlow
  session.
- **Gateway hooks** — the payload is `POST`ed to OpenClaw's documented
  [automation webhooks](https://docs.openclaw.ai/automation/cron-jobs#webhooks),
  `/hooks/wake` (enqueue a system event for the main session) or `/hooks/agent`
  (run an isolated agent turn).

TaskFlow delivery is configured per `route`; the `wake` and `agent` gateway hooks
are configured once at the top level (there is a single gateway). The three are
independent Svix Polling Endpoints, so you point one poller at a `wake` endpoint,
another at an `agent` endpoint, and one or more routes at TaskFlow endpoints.

## Why

OpenClaw's webhook surfaces are **push**-based: the bundled `webhooks` extension
(`extensions/webhooks/`) registers an inbound HTTP route for TaskFlow actions,
and the gateway exposes `/hooks/wake` + `/hooks/agent` for automation. Both
require the host to be reachable — a public URL, an open port, or a tunnel.

Many deployments can't (or won't) expose an inbound server: agents behind NAT,
locked-down networks, or setups that already have a durable message buffer in
front of them. This plugin inverts the direction. Nothing listens; background
pollers read [Svix Polling Endpoints](https://docs.svix.com/) — you give each
poller a **Polling Endpoint URL** and token — using the official
[`svix`](https://www.npmjs.com/package/svix) SDK, and deliver each buffered
message's payload to OpenClaw exactly as an inbound `POST` would.

## How it maps onto OpenClaw's webhook systems

```
                 OpenClaw (push)                          this plugin (pull)
                 ───────────────────────────────          ──────────────────────────────
  transport      inbound HTTP route                        Svix SDK poll loop
  auth           presented shared secret / hooks.token     Svix SDK Bearer auth (token)
  ─────────────────────────────────────────────────────────────────────────────────────
  TaskFlow       POST <webhooks route>                      poll `url`  → executeWebhookAction
                 webhookActionSchema → execute → classify   (validate → apply → classify)
  ─────────────────────────────────────────────────────────────────────────────────────
  wake           POST /hooks/wake   { text, mode }          poll `wake.url`  → POST /hooks/wake
  agent          POST /hooks/agent  { message, … }          poll `agent.url` → POST /hooks/agent
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
| `src/poller.ts` | The polling transport: Svix SDK poll loop over the configured Polling Endpoint URL → hands each message to a `dispatch` callback → advances the cursor. |
| `src/processor.ts` | Transport-agnostic abstraction over the vendored TaskFlow core (`validate → execute → classify`). |
| `src/vendor/webhook-actions.ts` | **Vendored from OpenClaw.** Action schemas, TaskFlow execution, and result mapping, copied verbatim. |
| `api.ts`, `runtime-api.ts` | Re-export shims for the `openclaw/plugin-sdk/*` SDK, mirroring the upstream extension. |

## Setup

There are two sides to wire up: one or more **Svix Ingest sources + Polling
Endpoints** (the buffers messages land in), and the **OpenClaw plugin** (the
pollers that drain them).

Create **one Polling Endpoint per destination** you want to feed:

- a TaskFlow endpoint → route `url`
- a wake endpoint → top-level `wake.url` → `POST /hooks/wake`
- an agent endpoint → top-level `agent.url` → `POST /hooks/agent`

The `wake`/`agent` destinations also require OpenClaw's gateway hooks to be
enabled (`hooks.enabled: true` + a `hooks.token`) — see step 3.

### 1. Create a Svix Ingest source and a Polling Endpoint

For each destination you need two values — the **Polling Endpoint URL** and an
**endpoint-scoped token** (`sk_endp_*`). You get them by standing up an Ingest
source and adding a Polling Endpoint as its destination. Repeat once per
destination (TaskFlow / wake / agent).

**Via the Svix Portal** (recommended for first setup):

1. In the [Svix dashboard](https://dashboard.svix.com) go to **Svix Ingest →
   Sources → Create source**. Name it (e.g. `openclaw`).
2. Pick the **Source Type**. Use **Generic Webhook** when your own automation
   produces the payloads; pick a provider (GitHub, Stripe, …) to have Svix verify
   that provider's signatures (then enable authentication and store the secret).
3. Copy the source's **Ingest URL** — the public URL events are `POST`ed to. Hand
   it to whatever produces the events (your automation, a provider webhook, etc.).
4. Open the source's **Destinations** tab → **Add Endpoint**, then change the type
   from **Webhook** to **Polling Endpoint** → **Create**. Unlike a push endpoint,
   a Polling Endpoint isn't given a URL of yours — it buffers events for a client
   to pull.
5. Open the new Polling Endpoint and click **Create API key** to mint its
   `sk_endp_*` token. Copy that token and the endpoint's **URL** (e.g.
   `https://api.eu.svix.com/api/v1/app/app_…/poller/poll_…`). The full URL and
   token are all the plugin needs — they go straight into `url` and `token` below.

**Via the Svix CLI** — handy for managing sources and grabbing the **ingest URL**:

```bash
brew install svix/svix/svix-cli      # or see https://docs.svix.com/cli
svix login                           # configure your API credentials once
svix ingest source list              # list sources
svix ingest source get <source_id>   # shows the source's ingestUrl
```

Note: `svix ingest source get` returns only the inbound `ingestUrl`. The
**Polling Endpoint URL and its `sk_endp_*` token are not exposed by the ingest
CLI** — create the Polling Endpoint and its API key in the portal (step 4–5
above) and copy them from there.

### 2. Install the plugin into OpenClaw

Link this plugin directory into your OpenClaw install (adds the load path and a
`plugins.entries.svix-openclaw` entry):

```bash
openclaw plugins install --link /path/to/svix/ai/plugins/svix-openclaw
```

### 3. Configure routes (and enable hooks for wake/agent)

Add TaskFlow routes under `plugins.entries.svix-openclaw.config.routes`, and the
`wake`/`agent` pollers alongside them at `plugins.entries.svix-openclaw.config`
(see the field reference in [Configuration](#configuration) below), using the poll
endpoint `url`(s) and polling token(s) from step 1. Store tokens via env secret
refs rather than inline.

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
`[svix-openclaw] polling Svix app=app_… sink=poll_… -> agent (poller agent)`.
Then send a test payload to the Ingest URL whose Polling Endpoint feeds that
poller:

```bash
# TaskFlow endpoint
curl -X POST "$TASKFLOW_INGEST_URL" -H 'Content-Type: application/json' \
  -d '{ "action": "create_flow", "goal": "Investigate alert" }'

# wake endpoint  -> /hooks/wake
curl -X POST "$WAKE_INGEST_URL" -H 'Content-Type: application/json' \
  -d '{ "text": "New email received", "mode": "now" }'

# agent endpoint -> /hooks/agent
curl -X POST "$AGENT_INGEST_URL" -H 'Content-Type: application/json' \
  -d '{ "message": "Summarize inbox", "name": "Email" }'
```

Each payload lands in its Polling Endpoint buffer; the matching poller reads it on
its next poll and either applies the TaskFlow action or POSTs it to the gateway
hook. A successful dispatch logs `dispatched … -> 2xx`.

## Configuration

Configured under `plugins.entries.svix-openclaw.config` in your OpenClaw config.
Each entry in `routes` is a TaskFlow poller (`url` + `token` + `sessionKey`). The
`wake` and `agent` pollers sit alongside `routes`, one of each. Configure at least
one poller (a route, `wake`, or `agent`) — with none, the plugin loads but starts
no pollers and does nothing.

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
              "url": "https://api.svix.com/api/v1/app/app_xxx/poller/poll_taskflow",
              "token": { "source": "env", "provider": "env", "id": "SVIX_TASKFLOW_TOKEN" },
              "sessionKey": "agent:main",
              "controllerId": "svix-openclaw/ops"
            }
          },

          // wake poller (optional): payloads POSTed to /hooks/wake.
          "wake": {
            "url": "https://api.svix.com/api/v1/app/app_xxx/poller/poll_wake",
            "token": { "source": "env", "provider": "env", "id": "SVIX_WAKE_TOKEN" }
          },

          // agent poller (optional): payloads POSTed to /hooks/agent.
          "agent": {
            "url": "https://api.svix.com/api/v1/app/app_xxx/poller/poll_agent",
            "token": { "source": "env", "provider": "env", "id": "SVIX_AGENT_TOKEN" },
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
| `url` | ✅ | — | Full TaskFlow Polling Endpoint URL, copied from Svix. |
| `token` | ✅ | — | Svix token for the TaskFlow `url`. Inline string or `{ source, provider, id }` secret ref. |
| `sessionKey` | ✅ | — | TaskFlow session the actions are applied to. |
| `controllerId` | | `svix-openclaw/<routeId>` | Controller id stamped on managed flows. |
| `enabled` | | `true` | Set `false` to skip the whole route. |

The TaskFlow poll-tuning fields below (`eventType`, `channel`, `pollIntervalMs`,
`limit`, `startIterator`, `payloadField`) also apply at the route level to the
TaskFlow `url`.

<a name="hook-endpoint-fields"></a>**Hook endpoint fields** (`wake` / `agent`)

| Field | Required | Default | Meaning |
| --- | --- | --- | --- |
| `url` | ✅ | — | Svix Polling Endpoint URL for this hook's messages. |
| `token` | ✅ | — | Svix token for this endpoint. Inline string or a secret ref. |
| `eventType` | | — | Svix-side event-type filter passed to the poll. |
| `channel` | | — | Svix-side channel filter passed to the poll. |
| `pollIntervalMs` | | `5000` | Idle wait after the endpoint reports `done: true`. |
| `limit` | | `50` | Page size per poll. |
| `startIterator` | | — | Resume cursor for the first poll. |
| `payloadField` | | `payload` | Field on each Svix message holding the body. Empty string ⇒ the whole message. |

> Auth + base URL for the hook `POST`s are read from your OpenClaw `hooks.token`
> and `gateway.port` — set `hooks.enabled: true` and a `hooks.token` (see
> [Setup step 3](#3-configure-a-route-and-enable-hooks-for-wakeagent)).

### Message payloads

Each poller polls its configured Polling Endpoint URL (`{ limit, iterator, … }`)
and walks the returned `PollingEndpointOut` (`data[]`, `iterator`, `done`),
advancing the cursor and idling for `pollIntervalMs` once `done` is `true`.

Each Svix message's `payload` is used verbatim as the request body for that
poller's destination, so the payload shape depends on which poller buffered it.

**TaskFlow (`url`)** — a webhook action object, the same JSON the upstream HTTP
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
