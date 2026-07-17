# Installation

Svix has two MCP servers. Pick the one that matches what you're doing:

| Server         | Use it to                                                                                                                                 | URL                              | Token from                         |
| -------------- | ----------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------- | ---------------------------------- |
| **App portal** | Debug the webhooks you **send** for one application: endpoints, failed attempts, the responses customers returned, resending and recovering. | `https://mcp.svix.com/app/<app_id>` | **App Portal** → **Get MCP Token** |
| **Ingest**     | Set up and debug the webhooks you **receive** from providers (Stripe, GitHub, …): sources, ingest URLs, handler endpoints, signing secrets. | `https://mcp.svix.com/ingest`    | **Dashboard** → **Ingest** → **Connect to MCP** |

Both are wired up the same way, and the ingest server includes the app portal's
debugging tools (taking the ingest source they apply to), so you don't need both
to debug an ingest handler.

## Step 1: Get your token

Click the button above for your server and pick your coding agent for
ready-to-paste setup steps. There's no separate login or OAuth flow to complete —
the token is all you need. The underlying config looks like this:

```json
{
  "mcpServers": {
    "<slug>-webhooks": {
      "url": "https://mcp.svix.com/app/<app_id>",
      "headers": {
        "Authorization": "Bearer <YOUR_TOKEN>"
      }
    }
  }
}
```

The app portal token is app-scoped and also encodes the app id, so nothing else
is needed; the `<app_id>` segment in the URL just keeps server URLs distinct so
you can connect clients for several Svix applications without them colliding. The
ingest token is scoped to your organization, and its URL takes no segment — its
tools name the ingest source they act on.

Both places name the server `<slug>-webhooks` — a readable label derived from
your display name (e.g. `acme-webhooks`) — so it's easy to recognize in your
client.

> Treat the token like a password. Don't commit it or share it.

The rest of this guide refers to the URL as `<YOUR_MCP_URL>` and the token as
`<YOUR_TOKEN>`, and uses the server name `<slug>-webhooks` (shortened to
`svix-debug` in the examples below). The token is always sent as an
`Authorization: Bearer <YOUR_TOKEN>` header.

## Step 2: Add the server to your coding agent

### Claude Code

```bash
claude mcp add --transport http svix-debug "<YOUR_MCP_URL>" \
  --header "Authorization: Bearer <YOUR_TOKEN>"
```

Verify it loaded with `/mcp`. See the [Claude Code MCP docs](https://docs.claude.com/en/docs/claude-code/mcp).

### Cursor

Open Settings (`Cmd/Ctrl` + `Shift` + `J`), go to **Tools & Integrations**,
select **New MCP Server**, and add (the App Portal's Cursor tab copies this same
config, with the server named `<slug>-webhooks`):

```json
{
  "mcpServers": {
    "svix-debug": {
      "url": "<YOUR_MCP_URL>",
      "headers": {
        "Authorization": "Bearer <YOUR_TOKEN>"
      }
    }
  }
}
```

### VSCode

1. `Cmd/Ctrl` + `P`, run **MCP: Add Server**.
2. Select **HTTP (HTTP or Server-Sent Events)**.
3. Enter `<YOUR_MCP_URL>`, then the name **svix-debug**.
4. Open `mcp.json` and add the auth header to the server entry:
   ```json
   {
     "servers": {
       "svix-debug": {
         "url": "<YOUR_MCP_URL>",
         "headers": { "Authorization": "Bearer <YOUR_TOKEN>" }
       }
     }
   }
   ```
5. Run **MCP: List Servers**, select **svix-debug**, **Start Server**.

### Codex

Edit `~/.codex/config.toml`:

```toml
[mcp_servers.svix-debug]
url = "<YOUR_MCP_URL>"
http_headers = { Authorization = "Bearer <YOUR_TOKEN>" }
```

### Gemini CLI

Edit `~/.gemini/settings.json` and restart:

```json
{
  "mcpServers": {
    "svix-debug": {
      "url": "<YOUR_MCP_URL>",
      "headers": {
        "Authorization": "Bearer <YOUR_TOKEN>"
      }
    }
  }
}
```

### OpenCode

Edit `~/.config/opencode/opencode.json` and restart:

```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "svix-debug": {
      "type": "remote",
      "url": "<YOUR_MCP_URL>",
      "headers": {
        "Authorization": "Bearer <YOUR_TOKEN>"
      }
    }
  }
}
```

### Zed

`Cmd/Ctrl` + `,` to open settings, then add:

```json
{
  "context_servers": {
    "svix-debug": {
      "url": "<YOUR_MCP_URL>",
      "headers": {
        "Authorization": "Bearer <YOUR_TOKEN>"
      },
      "settings": {}
    }
  }
}
```

### Other agents (Amp, Warp, Windsurf, ...)

Any client that supports a remote HTTP MCP server works. Point it at
`<YOUR_MCP_URL>` and send the `Authorization: Bearer <YOUR_TOKEN>` header.

## Alternative: run it locally (stdio)

Build the binary (see the [README](./README.md#build)) and pass a raw Svix API
token in the environment. `MCP_SERVER` picks which server to run.

App portal — an app-scoped token plus the application to debug:

```json
{
  "mcpServers": {
    "svix-debug": {
      "command": "/path/to/mcp/target/release/svix-mcp",
      "env": {
        "SVIX_TOKEN": "testsk_...",
        "SVIX_APP_ID": "app_...",
        "SVIX_CUSTOMER_NAME": "Acme"
      }
    }
  }
}
```

Ingest — an org-scoped token, no application (the tools name the source):

```json
{
  "mcpServers": {
    "svix-ingest": {
      "command": "/path/to/mcp/target/release/svix-mcp",
      "env": {
        "MCP_SERVER": "ingest",
        "SVIX_TOKEN": "testsk_...",
        "SVIX_CUSTOMER_NAME": "Acme"
      }
    }
  }
}
```

> Tools that create, update, delete, or rotate change live configuration, and
> `resend_message` / `recover_endpoint` perform real webhook deliveries. Only
> invoke them when you mean to.
