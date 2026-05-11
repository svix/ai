# Svix Agent Skills

A single Agent Skill that teaches Claude, Cursor, and other AI coding agents how to integrate [Svix](https://www.svix.com/) the way Svix's own engineers would: SDK-first, official-library-only signature verification, one Application per customer, and `svix listen` for local development.

```typescript
import { Svix } from "svix";
const svix = new Svix(process.env.SVIX_AUTH_TOKEN!);

// One-time per customer
await svix.application.create({ name: "Acme Inc.", uid: "customer_acme" });

// One per real domain event — Svix signs, fans out, retries
await svix.message.create("customer_acme", {
  eventType: "invoice.paid",
  payload: { invoiceId: "inv_123", amount: 9900, currency: "USD" },
});
```

That's [Svix Dispatch](https://www.svix.com/) — outbound. [Svix Ingest](https://www.svix.com/ingest/) is the same model in reverse: Svix exposes a public URL with built-in verification for Stripe / GitHub / Shopify / Slack / HubSpot / generic providers, then fans out to your handler. The [Svix CLI](https://docs.svix.com/tutorials/cli) wraps both, and adds `svix listen` for local relay.

## What the skill teaches

The repo ships one skill, `svix-best-practices`, with [layered references](skills/svix-best-practices/SKILL.md):

- **Dispatch** — sending: tenancy modeling, `message.create`, `Idempotency-Key`, [App Portal](https://docs.svix.com/app-portal) embedding, channels, polling endpoints, operational webhooks, regions, static IPs. Trigger phrases: "send webhooks to my customers", "embed the App Portal", "verify Svix signature in handler".
- **Ingest** — receiving: [Source Types](https://docs.svix.com/ingest/receiving-with-ingest) (provider presets), endpoints + fanout, JS filters and transformations, handlers that verify the Svix signature on fanout. Trigger phrases: "receive Stripe/GitHub/Shopify webhooks", "route inbound events to multiple consumers".
- **CLI** — local relay (`svix listen`), one-off provisioning, `jq` scripting. Trigger phrases: "test webhooks locally", "create an Application from a script".

The SKILL.md loads when the skill activates and links to per-topic references that load only when the agent needs them.

## Design choices

What this repo deliberately does, that distinguishes it from "skills that wrap a webhook product":

- **Library-only verification.** No manual HMAC code anywhere. The official [`svix` library](https://docs.svix.com/receiving/verifying-payloads/how) handles the `whsec_…` secret format, `v1,<base64>` signatures, multiple signatures during rotation, timestamp tolerance, and constant-time comparison. We link to the docs for the rare manual case and refuse to copy-paste the algorithm.
- **SDK-first.** Quickstarts and code examples lead with TypeScript / Python / Go / Ruby SDK calls. The CLI is for operators and scripts; production code uses the SDK in your language.
- **One Application per customer, with your own `uid`.** A core stance the skill enforces on every Dispatch question, so tenancy doesn't get reinvented per integration.
- **No example apps.** Skills + references only. Code samples are inline; full applications like the [Fullstack Next Example App](https://github.com/svix/svix-example) belong in the Svix samples repos, not here.
- **Concept-named references**, not numbered (`tenancy.md`, `app-portal.md`, `handlers.md`) — the work is rarely strictly sequential, and numbered prefixes imply a stage gate that doesn't really exist.

## Documentation links the skill cites

The skill always links inline to live docs as the source of truth — the list below is the index, but you don't need it day-to-day.

- Concepts and quickstart: <https://docs.svix.com/>
- Receiving (Ingest): <https://docs.svix.com/receiving>
- Signature verification: <https://docs.svix.com/receiving/verifying-payloads/how>
- App Portal: <https://docs.svix.com/app-portal>
- CLI: <https://docs.svix.com/tutorials/cli>
- API reference: <https://api.svix.com/docs>
- Server SDKs: <https://docs.svix.com/quickstart>

## Install

```bash
npx skills add svix/svix-agent-skills
```

The skill follows the [Agent Skills specification](https://agentskills.io/) — `name` and `description` load at startup, `SKILL.md` loads when the skill activates, reference files load on demand.

## Contributing

PRs welcome. See [skills/svix-best-practices/SKILL.md](skills/svix-best-practices/SKILL.md) for the authoring conventions — what belongs in SKILL.md vs. references, terminology, and the core stances the skill enforces.

## License

MIT
