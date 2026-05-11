# Dispatch

Outbound webhooks: your service sends events to your customers' endpoints via `message.create`. Svix signs, fans out, retries.

Live docs are the source of truth — see "Reference URLs" at the bottom and verify version-sensitive details (SDK signatures, API shapes) before answering.

## Tenancy and identifiers

- **One customer = one Application** is the default model. Map each customer to a stable [Application UID](https://docs.svix.com/overview#applications) (`customer_acme`), not the Svix-generated `app_…`.
- **Use Channels only for sub-tenancy within one customer** (e.g. `staging` / `production` for the same customer, reseller downstream tenants). If the same problem fits separate Applications, prefer that.
- **Don't overload Channels for things that should be Event Types.** "What kind of event" is an Event Type; "which slice of the same kind" is a Channel.
- **Channels are unvalidated strings** — typos silently route Messages nowhere. Coordinate names with the customer (they can edit channels in the App Portal).

## Auth and embedding

- **`SVIX_AUTH_TOKEN` is server-side only.** Never ship it in client bundles or commit it. It authenticates your service to the Svix management API (`message.create`, application/endpoint provisioning); leaking it lets anyone send messages as you.
- **App Portal for customer-facing UI.** Embed via a server-generated [session URL](https://docs.svix.com/app-portal). Never expose the management API or `SVIX_AUTH_TOKEN` to the browser.

## Idempotency

- **Always set `Idempotency-Key` on `message.create`.** Without it, every queue retry, network blip, or outbox replay creates a duplicate Message and delivers an extra time to every customer Endpoint.
- **Derive the key deterministically from the domain event.** Same event → same key; different events → different keys. Good shape: `<event-name>-<entity-id>-<event-version-or-timestamp>`, e.g. `invoice-paid-inv_123-2026-05-11T12:00:00Z`. ≤ 256 chars, ASCII.
- **Use the outbox pattern for at-least-once delivery.** In the same DB transaction as the domain mutation, insert into an `outbox` table with the deterministic key; a background worker drains it and calls `message.create`. Crashes between sending and marking sent are safe — Svix dedupes server-side. This is the canonical "no event lost, no event duplicated" pattern.
- **Don't confuse `Idempotency-Key` with `svix-id`.** `Idempotency-Key` is a request header **you** set on `message.create` (prevents you from creating duplicates). `svix-id` is a response header **your customer** sees on delivered webhooks (lets them dedupe retries).

### Idempotency traps

- **Random UUIDs.** A fresh UUID per call defeats the mechanism — the retry generates a new UUID and creates a duplicate. Never `crypto.randomUUID()`.
- **Reusing one key with different payloads.** Svix returns the first payload's stored response; the second payload is silently dropped.
- **Using only the entity id.** `invoice-${invoice.id}` collides when the entity changes state twice. Include version or timestamp.
- **Per-process counters.** Two app instances generate overlapping counters and cause false-positive idempotency hits.

## Operational webhooks

[Operational webhooks](https://docs.svix.com/incoming-webhooks) tell **your** infrastructure when something interesting happens to a customer's Endpoint. Wire them — without them you find out about broken endpoints when the customer complains.

- **Verify with the operational endpoint's own `whsec_…` secret** — official `svix` library, `svix-id` / `svix-timestamp` / `svix-signature` headers.
- **Don't confuse the operational secret with the customer's Endpoint secret.** Operational webhooks go to **your** infrastructure with **your** `whsec_…`. Customer Endpoints use **their** `whsec_…`. Different things, different env vars.
- **Don't double-handle.** A single failing customer Endpoint fires `message.attempt.failing` repeatedly. Layer a per-Endpoint "don't page again for 30 minutes" rule on top of `svix-id` dedupe.

Events worth wiring (always check the [live list](https://api.svix.com/docs#tag/Webhook) — Svix adds events):

| Event | Meaning | Route to |
|-------|---------|----------|
| `message.attempt.failing` | Endpoint hit consecutive failures, at risk of disable | On-call alerting |
| `endpoint.disabled` | Svix auto-disabled an Endpoint after sustained failure | Customer success |
| `endpoint.created` / `updated` / `deleted` | Customer changed config in the App Portal | Audit log |
| `message.attempt.exhausted` | All retries used up on one Attempt | Dead-letter queue |

## Inspect and replay

- **Single Attempt** — Dashboard/App Portal **Resend** button, or `svix message-attempt resend <app> <msg> <ep>`.
- **Bulk** — use the [recover endpoint](https://api.svix.com/docs#tag/Endpoint/operation/v1.endpoint.recover) for "all failed since X" rather than looping over `resend`.
- Replay creates a **new** Attempt; original history is preserved.
- Svix auto-disables Endpoints after sustained failure (thresholds at <https://docs.svix.com/retries>). Use the `endpoint.disabled` operational webhook to surface this to customer success.

## Triage cheatsheet

| Symptom | Likely cause |
|---------|--------------|
| All Endpoints failing immediately | Bad payload schema, token revoked, customer DNS down |
| One Endpoint failing while others succeed | Customer's URL changed or auth tightened |
| 4xx from customer | Handler rejecting body shape |
| 5xx from customer | Customer infra issue — Svix retries |
| `Idempotency-Key` reused with different payloads | API returned the **first** response |
| 401 from `message.create` | `SVIX_AUTH_TOKEN` invalid or missing |
| 404 on `app/{uid}/msg/` | App `uid` mismatch — confirm with `svix application get <uid>` |

## Reference URLs

Don't guess doc URLs — use these.

| Topic | URL |
|-------|-----|
| Overview / concepts | <https://docs.svix.com/overview> |
| Quickstart (per-language SDK install + signatures) | <https://docs.svix.com/quickstart> |
| Event Types | <https://docs.svix.com/event-types> |
| Consumer App Portal | <https://docs.svix.com/app-portal> |
| Verifying webhooks (official lib) | <https://docs.svix.com/receiving/verifying-payloads/how> |
| Verifying webhooks (manual fallback) | <https://docs.svix.com/receiving/verifying-payloads/how-manual> |
| Idempotency | <https://docs.svix.com/idempotency> |
| Retries | <https://docs.svix.com/retries> |
| Throttling | <https://docs.svix.com/throttling> |
| Rate limits | <https://docs.svix.com/rate-limit> |
| Static IPs | <https://docs.svix.com/security#ip-allow-list> |
| Operational webhooks | <https://docs.svix.com/incoming-webhooks> |
| Transformations (outbound) | <https://docs.svix.com/transformations> |
| Channels | <https://docs.svix.com/channels> |
| Polling endpoints | <https://docs.svix.com/polling-endpoints> |
| Interactive API docs | <https://api.svix.com/docs> |
| OpenAPI spec | <https://api.svix.com/api/v1/openapi.json> |
