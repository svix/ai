# Svix Agent Skills

[Agent Skills](https://agentskills.io/) that teach Claude, Cursor, and other AI coding agents how to integrate [Svix](https://www.svix.com/) the way Svix's own engineers would.

## Skills

- **`svix-best-practices`**: If you're maintaining an existing Svix integration, use this skill to ensure your agent uses Svix the way it's intended to be used. Covers [Dispatch](https://www.svix.com/) (sending webhooks: tenancy, `message.create`, idempotency, [App Portal](https://docs.svix.com/app-portal)), [Ingest](https://www.svix.com/ingest/) (receiving webhooks: Source Types, fanout, transformations, handlers), and the [Svix CLI](https://docs.svix.com/tutorials/cli) (`svix listen`, scripting).
- **`svix-integration-plan`**: Investigates the repo, asks the handful of questions Claude can't answer on its own (Application UID, event types, [App Portal](https://docs.svix.com/app-portal) vs custom UI, migration off an existing webhook system), then produces a written integration plan. Use for non-trivial integrations such as multi-tenant routing, event catalog, [Ingest](https://www.svix.com/ingest/) Sources before you code anything.
- **`svix-quickstart`**: Get a working Svix integration sending its first webhook as fast as possible. API key, SDK install, create an Application, send a message, subscribe customers. Use when adding [Svix](https://www.svix.com/) to a project for the first time and you just need the setup steps, not an architecture pass.

`SKILL.md` loads when a skill activates; per-topic references load on demand.

## Install

```bash
npx skills add svix/svix-agent-skills
```

## License

MIT