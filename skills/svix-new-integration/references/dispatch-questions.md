# Dispatch questions

Use these after triage when the user is sending webhooks to their customers or partners. Ask via `AskUserQuestion`, one topic at a time.

## 1. Event taxonomy

> **Do you already have an event catalog (a defined list of event names like `invoice.paid`, `user.created`), or are you designing it as part of this integration?**

- **Existing catalog** — capture it. The plan will pre-create those Event Types in Svix (CSV/JSON load or OpenAPI import).
- **Designing now** — Svix strongly recommends period-delimited names (`<group>.<event>`); the App Portal groups subscription UI by the prefix. Defer the full list to implementation; the plan should just note the convention.
- **Mixed** — some defined, some TBD. List what's known.

### Follow-up — schemas

> **Do you want JSONSchema definitions for each event type (helps customers, surfaces in the Portal), or skip schemas for now?**

- **With schemas** — recommended; use Draft 7. Note: Svix does **not** enforce schemas at send time, so this is for documentation/Portal UX, not validation.
- **Skip for now** — fine to start, add later. Capture it as a TODO in the plan.

## 2. Sub-tenancy within one customer (Channels)

> **Within a single customer, do you need to deliver only a slice of events to a specific endpoint — for example, "Production webhooks for repo A go here, repo B goes there," or "Staging vs production within the same customer"?**

- **Yes — per-project / per-repo / per-environment slicing** — use Channels. Capture the channel name shape (e.g. `repo:{name}`).
- **Yes — but the slices are really different kinds of events** — that's Event Types, not Channels. Don't conflate them.
- **No** — skip Channels entirely. Most integrations don't need them.

> Channels are unvalidated strings — typos silently route nowhere. The plan must include where channel names are generated and how typos are prevented.

## 3. Idempotency strategy

> **How does your service produce domain events today — synchronously in the request handler, via an outbox table, via a queue (SQS/Pub/Sub/Kafka), or some mix?**

- **Outbox table already exists** — perfect. Plan: insert outbox row in the same DB transaction as the domain mutation, drain worker calls `message.create` with `Idempotency-Key` derived from the outbox row id + event version.
- **Queue (SQS/Kafka/Pub/Sub)** — at-least-once delivery from the queue; the consumer calls `message.create` with a deterministic key. Plan must specify the key shape.
- **Synchronous in request handler** — works for low-volume, but every retry creates a duplicate without idempotency. Plan should recommend introducing an outbox or queue before going to production.
- **TBD** — flag as a blocker. No code should ship without an idempotency story.

### Follow-up — key shape

> **What identifiers are stable per domain event?** (e.g. `invoice_id` + `state_change_timestamp`)

Capture the exact fields. The plan will write `Idempotency-Key = <event-name>-<entity-id>-<version-or-timestamp>`, ≤256 ASCII chars.

## 4. Customer-facing UI

> **How will your customers configure their webhook Endpoints — through your own UI, the Svix App Portal (embedded), or both?**

- **Svix App Portal, embedded** — server generates a session URL per customer; embed via iframe or `svix-react`. Plan must include the session-URL endpoint and where in the product it links.
- **App Portal, with custom branding / access scope** — capture: light/dark mode, primary color, full vs `ViewBase` (read-only) capabilities, hide-navigation flag.
- **Custom UI calling the management API** — your backend wraps `endpoint.create/update/delete`. Never expose `SVIX_AUTH_TOKEN` to the browser; the plan must include a server-side proxy.
- **No customer-facing UI yet** — endpoints provisioned by your support team via dashboard or CLI. Capture this as a temporary state.

## 5. Operational webhooks

> **Do you want Svix to notify your infrastructure when a customer's endpoint starts failing, gets disabled, or is reconfigured?**

- **Yes — wire from day one** — recommended. Subscribe `message.attempt.failing`, `endpoint.disabled`, `endpoint.created/updated/deleted`, `message.attempt.exhausted`. Plan must specify destination (Slack, PagerDuty, audit log) per event.
- **Wire `endpoint.disabled` only** — minimum viable for customer-success awareness.
- **Defer** — accepted, but flag in the plan that you'll find out about broken endpoints from customer complaints.

> Operational webhooks use **your** `whsec_…` (not the customer's). The plan must call out which env var holds it and which handler verifies it.

## 6. Auth and secrets

> **Where will `SVIX_AUTH_TOKEN` live, and which environments need their own?**

- Capture: secret manager (Vault, AWS SM, Doppler, GCP SM), env var name, per-environment values, who has rotation access.
- The plan **must** state explicitly: `SVIX_AUTH_TOKEN` is server-side only. Never in frontend bundles, never in client envs, never in repo.


## 7. Migration / coexistence

> **Is there an existing webhook system being replaced, or is this greenfield?**

- **Greenfield** — easiest. Skip migration questions.
- **Replacing a homegrown sender** — capture: cutover strategy (dual-write, shadow, hard cutover), how subscribers are migrated, who owns notifying them.
- **Replacing another vendor** — capture event-name mapping (their names → yours), whether signatures change shape (almost certainly yes — customers need to re-verify).

## Output

When this branch finishes, you should be able to fill out every Dispatch field in <plan-template.md>. If any required field is still blank, ask one more targeted question before writing the plan.
