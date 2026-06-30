# Svix App Portal MCP

An [MCP](https://modelcontextprotocol.io) server (built with [`rmcp`](https://crates.io/crates/rmcp))
for debugging Svix webhook delivery problems: inspecting endpoints, drilling
into failed attempts, reading customer responses, and replaying messages.

Scoped to **one application** per session. The token is app-scoped and encodes
the app id.

## Connecting your coding agent

Grab a connection URL from the App Portal's **Get MCP Token** button and point
your agent at it. See [INSTALL.md](./INSTALL.md) for per-agent setup (Claude
Code, Cursor, VSCode, Codex, Gemini CLI, OpenCode, Zed, and others).

The sections below cover building and self-hosting the server.

## Build

```bash
cargo build --release
```

## Transports

Selected by `MCP_TRANSPORT`. In `stdio` mode the token and app id come from the
environment; in `http` mode the MCP token is read per-request from the
`Authorization: Bearer <token>` header (it also encodes the app id). The server
is mounted at `/mcp/{slug}`: the path segment is a cosmetic slug (app portal
display name, environment, and region) that the server ignores (it authenticates
entirely from the token) but that keeps URLs distinct so you can connect clients
for several Svix customers, environments, and regions without them colliding. A
request without a token gets a `401`.

| Variable          | Required   | Description                                                         |
| ----------------- | ---------- | ------------------------------------------------------------------ |
| `MCP_TRANSPORT`   | no         | `stdio` (default) or `http`.                                       |
| `SVIX_TOKEN`      | stdio only | Svix API token (region inferred from its suffix).                 |
| `SVIX_APP_ID`     | stdio only | The application id (or UID) this session debugs.                  |
| `MCP_BIND_ADDR`   | no         | HTTP bind address. Defaults to `127.0.0.1:8080`, at `/mcp/{slug}`.   |
| `SVIX_SERVER_URL` | no         | Override the API base URL (e.g. `http://localhost:8071`).         |
| `RUST_LOG`        | no         | Log filter (stderr). Defaults to `info`.                          |

See [INSTALL.md](./INSTALL.md) for client configuration examples for both
transports.

## Tools

| Tool                         | Purpose                                                                          |
| ---------------------------- | -------------------------------------------------------------------------------- |
| `get_application`            | The application this session is scoped to (name, UID, metadata).                  |
| `list_endpoints`             | List the application's endpoints (URL, enabled state, filtered event types).      |
| `get_endpoint`               | Full configuration of one endpoint.                                              |
| `get_endpoint_stats`         | Success / fail / pending / sending counts over a time window.                     |
| `get_transformation`         | An endpoint's transformation code, enabled state, and variables.                  |
| `update_transformation`      | Set an endpoint's transformation code and/or toggle it on/off (modifies config).  |
| `list_messages`              | List messages sent to the application (filter by event type, channel, time).      |
| `list_attempts_by_endpoint`  | Delivery attempts for an endpoint (filter `status=fail`), with response bodies.   |
| `list_attempts_by_message`   | Every endpoint a single message was attempted against and how each responded.     |
| `get_message`                | A message's event type, channels, and JSON payload.                              |
| `get_attempt`                | One attempt in full: response status code and body returned by the customer.      |
| `resend_message`             | Resend one message to an endpoint (real delivery).                               |
| `recover_endpoint`           | Replay all failed messages for an endpoint since a date (real deliveries).        |

> `resend_message` and `recover_endpoint` perform real deliveries, and
> `update_transformation` modifies the live endpoint configuration. Only invoke
> them when the user explicitly asks to resend, recover, or change the
> transformation.
