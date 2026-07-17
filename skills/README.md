# Svix Agent Skills

[Agent Skills](https://agentskills.io/) that teach Claude, Cursor, and other AI coding agents how to integrate [Svix](https://www.svix.com/) the way Svix's own engineers would. This folder is their canonical source.

## The skills

- **`svix-integration`**: everything for working with Svix, so the agent uses it as intended. It builds by default, routing on what you're doing: first-time setup (API key, SDK install, first message), [Dispatch](https://www.svix.com/) (sending webhooks: tenancy, `message.create`, idempotency, [App Portal](https://docs.svix.com/app-portal)), [Ingest](https://www.svix.com/ingest/) (receiving webhooks: Source Types, fanout, transformations, handlers), and the [Svix CLI](https://docs.svix.com/tutorials/cli) (`svix listen`, scripting). Ask it for a plan — and only if you ask — and it switches modes instead: investigate the repo, ask the handful of questions it can't answer on its own, write an integration plan, and stop before any code.
- **`receiving-webhooks`**: provider-agnostic guidelines for building a robust webhook receiver. Use when writing, reviewing, or debugging a handler that consumes incoming webhooks, from Svix or anyone else.

## How they load

Each skill is a `SKILL.md`, the instructions that enter the agent's context when the skill activates, plus a `references/` folder of per-topic files the agent pulls in on demand. `SKILL.md` is a router: it holds the decision, not the content. So `svix-integration` doesn't spend context on Ingest guidance while the agent is working on Dispatch, and doesn't carry the planning workflow while it's writing code.

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
