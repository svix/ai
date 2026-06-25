# Installation

The Svix App Portal MCP server lets your coding agent debug webhook delivery for
**one application**: inspecting endpoints, failed attempts, customer responses,
and replaying messages.

## Step 1: Get your token from the App Portal

Open the **App Portal** and click **Get MCP Token**. This gives you a connection
URL with an app-scoped token and app id baked in:

```
https://mcp.svix.com/mcp?token=<YOUR_TOKEN>&app_id=<YOUR_APP_ID>
```

Copy that URL. Every agent below uses it, and there's no separate login or OAuth
flow to complete.

> Treat the URL like a password. Don't commit it or share it.

The rest of this guide refers to it as `<YOUR_MCP_URL>` and names the server
`svix-debug`.

## Step 2: Add the server to your coding agent

### Claude Code

```bash
claude mcp add --transport http svix-debug "<YOUR_MCP_URL>"
```

Verify it loaded with `/mcp`. See the [Claude Code MCP docs](https://docs.claude.com/en/docs/claude-code/mcp).

### Cursor

Open Settings (`Cmd/Ctrl` + `Shift` + `J`), go to **Tools & Integrations**,
select **New MCP Server**, and add:

```json
{
  "mcpServers": {
    "svix-debug": {
      "url": "<YOUR_MCP_URL>"
    }
  }
}
```

### VSCode

1. `Cmd/Ctrl` + `P`, run **MCP: Add Server**.
2. Select **HTTP (HTTP or Server-Sent Events)**.
3. Enter `<YOUR_MCP_URL>`, then the name **svix-debug**.
4. Run **MCP: List Servers**, select **svix-debug**, **Start Server**.

### Codex

```bash
codex mcp add svix-debug --url "<YOUR_MCP_URL>"
```

Or edit `~/.codex/config.toml`:

```toml
[mcp_servers.svix-debug]
url = "<YOUR_MCP_URL>"
```

### Gemini CLI

Edit `~/.gemini/settings.json` and restart:

```json
{
  "mcpServers": {
    "svix-debug": {
      "url": "<YOUR_MCP_URL>"
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
      "url": "<YOUR_MCP_URL>"
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
      "settings": {}
    }
  }
}
```

### Other agents (Amp, Warp, Windsurf, ...)

Any client that supports a remote HTTP MCP server works. Point it at
`<YOUR_MCP_URL>`.

## Alternative: run it locally (stdio)

Build the binary (see the [README](./README.md#stdio-local-client)) and pass the
token and app id as environment variables:

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
