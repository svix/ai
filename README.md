# Svix for AI Agents

Everything Svix builds to make AI agents good at webhooks: teaching them how to [send and receive webhooks](https://www.svix.com/) the way Svix's own engineers would, giving them tools to debug a broken delivery, and letting them receive webhooks without hosting a public HTTP server.

Three kinds of things live here:

| | What it is | For |
| --- | --- | --- |
| [**Skills**](#skills) | [Agent Skills](https://agentskills.io/): instructions that load into the agent's context on demand | Coding agents (Claude, Cursor, …) writing or reviewing a Svix integration |
| [**MCP servers**](#mcp-servers) | Tools an agent can call against a live Svix account | Debugging real webhook deliveries from your editor |

This repo is itself an [Agent Plugins](https://agent-plugins.org/) package: the [`plugin.json`](plugin.json) at the root packages the skills below for any conformant client.

## Skills

Instructions an agent loads when it's about to touch Svix: planning an integration, wiring up the first webhook, reviewing an existing one, or writing a receiver. Two of them live in [`skills/`](skills/), and that README has the full rundown.

Install them into any project:

```bash
npx skills add svix/ai
```

## MCP servers

For more information, visit [our docs](https://docs.svix.com/ai/mcp)


## Personal Agent Plugins

Agent runtimes usually receive webhooks by exposing an inbound HTTP route, which means a public URL, an open port, or a tunnel. These plugins invert that: they **poll** a Svix sink with the SDK's [`AutoConfigConsumer`](https://docs.svix.com/receiving/webhooks-autoconfig) and hand each message to the runtime exactly as an inbound `POST` would. Nothing listens, so they work from a laptop behind NAT or a locked-down network, and the buffer in front means events survive a restart.

- **[`svix-openclaw`](svix-openclaw/)**: [OpenClaw](https://docs.openclaw.ai/) plugin. Polled payloads become TaskFlow actions, or get POSTed to the gateway's `/hooks/wake` and `/hooks/agent` automation hooks.
- **[`svix-hermes`](svix-hermes/)**: [Hermes Agent](https://github.com/NousResearch/hermes-agent) gateway plugin. Each event flows through a route, prompt, and delivery pipeline, with responses going to a log, a GitHub comment, or any connected platform.

Each plugin's README covers install and configuration.

## License

MIT
