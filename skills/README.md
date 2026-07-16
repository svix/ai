# Svix Agent Skills

[Agent Skills](https://agentskills.io/) that teach Claude, Cursor, and other AI coding agents how to integrate [Svix](https://www.svix.com/) the way Svix's own engineers would. This folder is their canonical source.

## The skills

- **`svix-quickstart`**: the fastest path to a working integration sending its first webhook. API key, SDK install, create an Application, send a message, subscribe customers. Use when adding Svix to a project for the first time and you just need the setup steps, not an architecture pass.
- **`svix-integration-plan`**: investigates the repo, asks the handful of questions the agent can't answer on its own (Application UID, event types, [App Portal](https://docs.svix.com/app-portal) vs custom UI, migration off an existing webhook system), then produces a written integration plan. Use for non-trivial integrations such as multi-tenant routing, an event catalog, or [Ingest](https://www.svix.com/ingest/) Sources, before you code anything.
- **`svix-best-practices`**: for maintaining an existing integration, so the agent uses Svix as intended. Covers [Dispatch](https://www.svix.com/) (sending webhooks: tenancy, `message.create`, idempotency, App Portal), [Ingest](https://www.svix.com/ingest/) (receiving webhooks: Source Types, fanout, transformations, handlers), and the [Svix CLI](https://docs.svix.com/tutorials/cli) (`svix listen`, scripting).
- **`receiving-webhooks`**: provider-agnostic guidelines for building a robust webhook receiver. Use when writing, reviewing, or debugging a handler that consumes incoming webhooks, from Svix or anyone else.

## How they load

Each skill is a `SKILL.md`, the instructions that enter the agent's context when the skill activates, plus, for the larger ones, a `references/` folder of per-topic files the agent pulls in on demand. So `svix-best-practices` doesn't spend context on Ingest guidance while the agent is working on Dispatch.

## Install

```bash
npx skills add svix/ai
```

## What else Svix gives an agent

Skills are the instruction layer. Alongside them:

- LLM-readable docs at <https://docs.svix.com/>. Append `.md` to any docs URL.
- Official server SDKs that handle signature verification, retries, idempotency, and sending.
- The [Svix CLI](https://docs.svix.com/tutorials/cli) (`npx svix-cli`) for scripting and `svix listen` (local relay).
- [MCP servers](../mcp/) for setting up and debugging live webhooks, and [agent plugins](../plugins/) that deliver webhooks into an agent runtime. See the [root README](../README.md).
