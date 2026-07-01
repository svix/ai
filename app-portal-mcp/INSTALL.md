# Installation

The Svix App Portal MCP server lets your coding agent debug webhook delivery for
**one application**: inspecting endpoints, failed attempts, customer responses,
and replaying messages.

## Step 1: Get your token from the App Portal

Open the **App Portal** and click **Get MCP Token**, then pick your coding agent
for ready-to-paste setup steps. Each one uses a server URL and an app-scoped token
(the token also encodes the app id, so nothing else is needed). The underlying
config looks like this:

```json
{
  "mcpServers": {
    "<slug>-webhooks": {
      "url": "https://mcp.svix.com/mcp/<slug>",
      "headers": {
        "Authorization": "Bearer <YOUR_TOKEN>"
      }
    }
  }
}
```

There's no separate login or OAuth flow to complete. The `<slug>` segment combines
your app portal display name, environment, and region
(e.g. `acme_production_us`): it keeps the server URL distinct so you can connect
clients for several Svix customers, environments, and regions without the URLs
colliding, and the portal names the server `<slug>-webhooks` for the same reason —
so MCP servers stay distinct in your client.

> Treat the token like a password. Don't commit it or share it.

The rest of this guide refers to the URL as `<YOUR_MCP_URL>`
(`https://mcp.svix.com/mcp/<slug>`) and the token as `<YOUR_TOKEN>`, and uses
the server name `<slug>-webhooks` the portal provides (shortened to
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

Build the binary (see the [README](./README.md#build)) and pass the token and app
id as environment variables:

```json
{
  "mcpServers": {
    "svix-debug": {
      "command": "/path/to/app-portal-mcp/target/release/app-portal-mcp",
      "env": {
        "SVIX_TOKEN": "testsk_...",
        "SVIX_APP_ID": "app_..."
      }
    }
  }
}
```

> `resend_message` and `recover_endpoint` perform real webhook deliveries. Only
> invoke them when you want to resend or recover.
