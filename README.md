# Svix Agent Skills

[Agent Skills](https://agentskills.io/) that teach Claude, Cursor, and other AI coding agents how to integrate [Svix](https://www.svix.com/) the way Svix's own engineers would.

## Skills

- **`svix-best-practices`**: Guidance for building, modifying, or reviewing any Svix integration. Covers [Dispatch](https://www.svix.com/) (sending webhooks: tenancy, `message.create`, idempotency, [App Portal](https://docs.svix.com/app-portal)), [Ingest](https://www.svix.com/ingest/) (receiving webhooks: Source Types, fanout, transformations, handlers), and the [Svix CLI](https://docs.svix.com/tutorials/cli) (`svix listen`, scripting).
- **`svix-new-integration`**: A discovery interview for starting a fresh integration. Walks the user through branching questions (direction, tenancy, identifiers, event taxonomy, idempotency, environments), then produces a written implementation plan before any code is written.

`SKILL.md` loads when a skill activates; per-topic references load on demand.

## Install

```bash
npx skills add svix/svix-agent-skills
```

## License

MIT