# Svix for AI Agents

Everything Svix builds to make AI agents good at webhooks: teaching them how to [send and receive webhooks](https://www.svix.com/) the way Svix's own engineers would, giving them tools to debug a broken delivery, and letting them receive webhooks without hosting a public HTTP server.

Three kinds of things live here:

| | What it is | For |
| --- | --- | --- |
| [**Skills**](#skills) | [Agent Skills](https://agentskills.io/): instructions that load into the agent's context on demand | Coding agents (Claude, Cursor, …) writing or reviewing a Svix integration |
| [**MCP servers**](#mcp-servers) | Tools an agent can call against a live Svix account | Debugging real webhook deliveries from your editor |
| [**Agent plugins**](#agent-plugins) | Platform plugins that deliver webhooks *to* an agent runtime | Running agents that wake up on webhook events |

## Skills

Instructions an agent loads when it's about to touch Svix: planning an integration, wiring up the first webhook, reviewing an existing one, or writing a receiver. Two of them live in [`skills/`](skills/), and that README has the full rundown.

Install them into any project:

```bash
npx skills add svix/ai
```

## MCP servers

Two [MCP](https://modelcontextprotocol.io) servers, one per side of a webhook, both in **[`mcp`](mcp/)**:

- **App portal** (`/app/{app_id}`): debug the webhooks you **send**. List endpoints, drill into failed attempts, read the response the customer's server actually returned, and resend or recover messages. It is scoped to a single application via an app-scoped token, so you can hand it to an agent (or a customer) without exposing the rest of your account.
- **Ingest** (`/ingest`): set up and debug the webhooks you **receive** from providers like Stripe or GitHub. Create a source and get the URL to register with the provider, point it at the handler you're writing, fetch the signing secret that handler verifies against, then read the payloads that actually arrived and the errors your handler returned. It inherits the app portal's debugging tools, applied to an ingest source.

Grab a connection URL from the App Portal's **Get MCP Token** button, or the Dashboard's Ingest page, and point your agent at it. See [`mcp/INSTALL.md`](mcp/INSTALL.md) for per-agent setup.

## Agent plugins

Agent runtimes usually receive webhooks by exposing an inbound HTTP route, which means a public URL, an open port, or a tunnel. These plugins invert that: they **poll** a Svix sink with the SDK's [`AutoConfigConsumer`](https://docs.svix.com/receiving/webhooks-autoconfig) and hand each message to the runtime exactly as an inbound `POST` would. Nothing listens, so they work from a laptop behind NAT or a locked-down network, and the buffer in front means events survive a restart.

- **[`svix-openclaw`](plugins/svix-openclaw/)**: [OpenClaw](https://docs.openclaw.ai/) plugin. Polled payloads become TaskFlow actions, or get POSTed to the gateway's `/hooks/wake` and `/hooks/agent` automation hooks.
- **[`svix-hermes`](plugins/svix-hermes/)**: [Hermes Agent](https://github.com/NousResearch/hermes-agent) gateway plugin. Each event flows through a route, prompt, and delivery pipeline, with responses going to a log, a GitHub comment, or any connected platform.

Each plugin's README covers install and configuration.

## License

MIT
