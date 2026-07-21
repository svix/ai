# Ingest questions

Use these when the user is receiving webhooks from third-party providers (or upstream services they control). Ask via `AskUserQuestion`, one topic at a time.

## Rules

- Each question gets a **plain-language explainer**. Assume the user hasn't read the Svix docs.
- **Pre-fill from the repo.** If you found a Stripe webhook handler, propose Stripe as the provider; don't ask blind.
- **One topic per `AskUserQuestion`.**

Ingest tenancy is different from Dispatch: a **Source** is the public URL that receives the upstream's webhooks; **Endpoints** under that Source fan the verified payload out to your internal services. There's no per-customer Application — per-tenant work happens in your handler after the fanout.

The goal is a working first integration. Operational webhooks, full secret-manager conventions, and complex dedupe schemes are out of scope — list them under **TODO** in the plan.

## 1. Providers

> **Which third parties will you receive webhooks from?**

Description text:

> A Svix **Source** is the public URL where a provider posts. For known providers (Stripe, GitHub, Shopify, Slack, Twilio, etc.) Svix has a built-in preset that handles their signature scheme — you never re-implement HMAC. For unknown providers, we use a generic source.

Capture the exact provider names. For each, verify a preset exists by checking <https://github.com/svix/svix-webhooks/blob/main/javascript/src/models/ingestSourceIn.ts#L4> or grepping `IngestSourceIn` in the installed SDK.

For providers without a preset:
- **If the upstream is your own service** → use the `svix` source type; generate a `whsec_…` secret in Svix and deploy it to the sender.
- **If it's a real third party with no preset** → use `generic-webhook`. Treat the Source URL as public and unauthenticated — your handler must validate the payload.

## 2. Per-tenant routing

> **Does every incoming webhook belong to one of your customers, and if so, how is that customer identified?**

Description text:

> If the webhook tells you which of your customers it's for, your handler reads that field and routes accordingly. If the webhook is global (no per-tenant data), you just have one Source and one Endpoint.

Options:

- **Yes, the provider includes a tenant identifier in the payload** (e.g. Stripe `account`, GitHub `installation.id`) — capture the exact field path.
- **Yes, but routing is by the provider's account, which is global** — one Source per upstream account; capture how many you need.
- **No, webhooks are global** — single Source, single Endpoint, no per-tenant logic.

## 3. Fanout

> **How many of your services need each verified webhook?**

Description text:

> Once Svix verifies the upstream's signature, it can fan the payload out to one or many internal Endpoints. Each Endpoint is one HTTPS URL in your infrastructure.

Options:

- **One service** — single Endpoint under the Source.
- **Several services, same payload** — one Endpoint per consuming service.
- **Several services, each wants a different subset** — fanout with per-Endpoint **filters**. Capture the filter criteria.
- **Several services, each wants a different shape** — fanout with per-Endpoint **transformations**. Note: transformations are stateless JS, no DB / external calls, no auth decisions. If logic doesn't fit, do it in the handler.

## 4. Handler

> **What language / framework will the consuming handler use?**

Description text:

> The handler verifies the Svix signature on each fanout, then runs your business logic. One footgun: signature verification requires the **raw request body**, so middleware that re-stringifies JSON breaks it.

Capture:
- **Language / framework** — pre-fill from the repo. The matching SDK has a one-call verification helper.
- **Raw body access** — confirm the framework can hand you bytes before JSON parsing (Express `express.raw`, FastAPI `Request.body()`, Go `io.ReadAll(r.Body)`). If unsure, flag as a blocker.

For languages without an SDK, fall back to <https://docs.svix.com/receiving/verifying-payloads/how-manual>.

## 5. Dedupe

> **Is your handler safe to run twice on the same payload?**

Description text:

> Svix delivers at-least-once. If your handler isn't already idempotent (e.g. upserts by domain id), you need a small dedupe table keyed by the `svix-id` header. Otherwise duplicate retries cause duplicate side effects.

Options:

- **Already idempotent** — note it in the plan.
- **Needs dedupe** — capture which table or store holds the processed-IDs set.
- **Don't know yet** — flag.

## Local development

Recommend `svix listen` for testing inbound webhooks against a developer's laptop — it gives you a public URL that forwards to a local port.

## Output

Fill in the Ingest section of <plan-template.md>. Operational webhooks for internal endpoints, full secret-manager conventions, and advanced transformation logic go under **out of scope / TODO**.
