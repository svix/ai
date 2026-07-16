# Svix MCP

Two [MCP](https://modelcontextprotocol.io) servers (built with
[`rmcp`](https://crates.io/crates/rmcp)), one binary:

| Server         | Mounted at      | For                                                                                       |
| -------------- | --------------- | ----------------------------------------------------------------------------------------- |
| **App portal** | `/app/{app_id}` | Debugging the webhooks you **send**: endpoints, failed attempts, resend, recover.          |
| **Ingest**     | `/ingest`       | Setting up and debugging the webhooks you **receive** from providers (Stripe, GitHub, …). |

The app portal server is scoped to **one application** per session — its token is
app-scoped and encodes the app id, so no tool takes an `app_id`.

The ingest server is scoped to the **organization**, so its tools name the ingest
source they act on. It **inherits the app portal's tools**: an ingest source is
backed by a Svix application, so the same message and attempt tools work against
it, taking a `source_id`. On top of those it adds the ingest-native tools
(sources, ingest endpoints, signing secrets).

## Connecting your coding agent

Grab a connection URL from the App Portal's **Get MCP Token** button and point
your agent at it. See [INSTALL.md](./INSTALL.md) for per-agent setup (Claude
Code, Cursor, VSCode, Codex, Gemini CLI, OpenCode, Zed, and others).

The sections below cover building and self-hosting the servers.

## Build

```bash
cargo build --release
```

## Transports

Selected by `MCP_TRANSPORT`. In `http` mode **both** servers are served and the
path picks one; the MCP token is read per-request from the `Authorization: Bearer
<token>` header. A request without a token gets a `401`. In `stdio` mode a single
server is served, picked by `MCP_SERVER`, and the token comes from the
environment.

The app portal's `{app_id}` path segment is ignored by the server (it
authenticates entirely from the token, which also encodes the app id); it keeps
URLs distinct so you can connect clients for several Svix applications without
them colliding. The ingest server needs no such segment.

| Variable             | Required   | Description                                                                                                                                 |
| -------------------- | ---------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| `MCP_TRANSPORT`      | no         | `stdio` (default) or `http`.                                                                                                                 |
| `MCP_SERVER`         | no         | stdio only. `app-portal` (default) or `ingest`.                                                                                              |
| `SVIX_TOKEN`         | stdio only | Svix API token (region inferred from its suffix). App-scoped for `app-portal`, org-scoped for `ingest`.                                      |
| `SVIX_APP_ID`        | stdio only | The application id (or UID) this session debugs. App portal only.                                                                            |
| `SVIX_CUSTOMER_NAME` | no         | stdio only. Customer/brand name (e.g. `Acme`) used to tailor the server's instructions and triggers. In http mode this comes from the token. |
| `MCP_BIND_ADDR`      | no         | HTTP bind address. Defaults to `127.0.0.1:8080`.                                                                                             |
| `SVIX_SERVER_URL`    | no         | Override the API base URL (e.g. `http://localhost:8071`).                                                                                    |
| `RUST_LOG`           | no         | Log filter (stderr). Defaults to `info`.                                                                                                     |

See [INSTALL.md](./INSTALL.md) for client configuration examples for both
transports.

## App portal tools

| Tool                        | Purpose                                                                          |
| --------------------------- | -------------------------------------------------------------------------------- |
| `get_application`           | The application this session is scoped to (name, UID, metadata).                  |
| `list_endpoints`            | List the application's endpoints (URL, enabled state, filtered event types).      |
| `get_endpoint`              | Full configuration of one endpoint.                                               |
| `get_endpoint_stats`        | Success / fail / pending / sending counts over a time window.                     |
| `get_transformation`        | An endpoint's transformation code, enabled state, and variables.                  |
| `update_transformation`     | Set an endpoint's transformation code and/or toggle it on/off (modifies config).  |
| `list_messages`             | List messages sent to the application (filter by event type, channel, time).      |
| `list_attempts_by_endpoint` | Delivery attempts for an endpoint (filter `status=fail`), with response bodies.   |
| `list_attempts_by_message`  | Every endpoint a single message was attempted against and how each responded.     |
| `get_message`               | A message's event type, channels, and JSON payload.                               |
| `get_attempt`               | One attempt in full: response status code and body returned by the customer.      |
| `resend_message`            | Resend one message to an endpoint (real delivery).                                |
| `recover_endpoint`          | Replay all failed messages for an endpoint since a date (real deliveries).        |

> `resend_message` and `recover_endpoint` perform real deliveries, and
> `update_transformation` modifies the live endpoint configuration. Only invoke
> them when the user explicitly asks to resend, recover, or change the
> transformation.

## Ingest tools

A **source** is one provider you receive webhooks from. It has an `ingestUrl` you
register with that provider, and a `type` (`stripe`, `github`, …, or
`generic-webhook`) whose `config` carries the provider's own signing secret so
Svix can verify what arrives. A source forwards what it receives to its **ingest
endpoints** — your handlers — signed with a Svix signature the handler verifies
using the endpoint's signing secret.

### Sources

| Tool                     | Purpose                                                                       |
| ------------------------ | ----------------------------------------------------------------------------- |
| `list_sources`           | The organization's ingest sources.                                             |
| `get_source`             | One source: type, provider config, and the `ingestUrl` to give the provider.   |
| `create_source`          | Create a source for a provider and get its `ingestUrl`.                        |
| `update_source`          | Replace a source's configuration (e.g. rotate the provider's signing secret).  |
| `delete_source`          | Delete a source, its endpoints, and its `ingestUrl` (destructive).             |
| `rotate_source_token`    | New `ingestUrl`; the old one keeps working for 48 hours.                       |
| `get_source_portal_link` | Magic link into the Ingest consumer portal UI for a source.                    |

### Ingest endpoints

| Tool                                    | Purpose                                                            |
| --------------------------------------- | ------------------------------------------------------------------ |
| `list_ingest_endpoints`                 | The URLs a source forwards to (your handlers).                      |
| `get_ingest_endpoint`                   | One endpoint's URL, enabled state, rate limit, and metadata.        |
| `create_ingest_endpoint`                | Point a source at a handler URL (starts real deliveries).           |
| `update_ingest_endpoint`                | Repoint or disable an endpoint (replaces its configuration).        |
| `delete_ingest_endpoint`                | Stop forwarding to an endpoint (destructive).                       |
| `get_ingest_endpoint_secret`            | The `whsec_...` secret the handler verifies signatures with.        |
| `rotate_ingest_endpoint_secret`         | New signing secret; the old one stays valid for 24 hours.           |
| `get_ingest_endpoint_headers`           | Custom headers sent with every delivery.                            |
| `update_ingest_endpoint_headers`        | Replace those headers.                                              |
| `get_ingest_endpoint_transformation`    | The transformation applied before the webhook reaches the handler.  |
| `update_ingest_endpoint_transformation` | Set its code and/or toggle it on/off (modifies config).             |

### Inherited from the app portal

These take the same arguments as their app portal counterparts, plus the
`source_id` they apply to. The server resolves the source to the application
backing it — by minting a consumer portal token, see
[`src/ingest/portal.rs`](./src/ingest/portal.rs) — and runs the same
implementation against it.

| Tool                        | Purpose in the ingest context                                                        |
| --------------------------- | ------------------------------------------------------------------------------------ |
| `list_messages`             | Webhooks the source received from the provider, with their payloads.                 |
| `get_message`               | One received webhook — the exact payload the handler has to parse.                   |
| `get_endpoint_stats`        | Health of an ingest endpoint (a high fail count means the handler is rejecting).      |
| `list_attempts_by_endpoint` | Attempts to deliver to a handler, with the status and body it returned.               |
| `list_attempts_by_message`  | Every endpoint one received webhook was attempted against.                            |
| `get_attempt`               | The exact error a failing handler produced.                                           |
| `resend_message`            | Replay one webhook to a handler, re-signed (unlike a captured payload replayed by curl). |
| `recover_endpoint`          | Replay everything that failed to reach a handler since a date.                        |

`get_application`, `list_endpoints`, `get_endpoint`, `get_transformation`, and
`update_transformation` are *not* inherited: the ingest-native tools above
address the same objects with ingest semantics.

> `create_*`, `update_*`, `delete_*`, `rotate_*`, `resend_message`, and
> `recover_endpoint` change live configuration or perform real deliveries. Only
> invoke them when the user explicitly asks.
