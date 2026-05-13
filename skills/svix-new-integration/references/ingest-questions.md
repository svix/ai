# Ingest questions

Use these after triage when the user is receiving webhooks from third-party providers (or upstream services they control). Ask via `AskUserQuestion`, one topic at a time.

Ingest tenancy is different from Dispatch: a **Source** is the public URL that receives the upstream's webhooks; **Endpoints** under that Source fan the verified payload out to your services. There's no per-customer Application unless your handler does per-tenant work after receiving the fanout.

## 1. Providers

> **Which providers will you receive webhooks from?**

- List the exact provider names (Stripe, GitHub, Shopify, Slack, Twilio, …).
- For each, the plan will check whether a **Source Type preset** exists (grep `IngestSourceIn` in the installed SDK or check <https://github.com/svix/svix-webhooks/blob/main/javascript/src/models/ingestSourceIn.ts>). Presets handle the provider's signature scheme at the edge — never re-implement HMAC if a preset exists.

### Follow-up — providers without a preset

For each provider on the list with no preset, decide:

- **Upstream is your own service** → use `svix` Source Type. Generate the `whsec_…` secret in Svix and deploy it to the sender.
- **Upstream is a real third party with no preset** → use `generic-webhook` (no signature verification at the edge) **and** treat the Source URL as a public, unauthenticated endpoint.
- **No signing at all** → `generic-webhook` is the only option. Plan must note the trust boundary: anything posted to that URL will be fanned out, so the handler must validate the payload itself.

## 2. Per-tenant routing

> **Does every upstream webhook belong to exactly one of your customers, and if so, how is that customer identified in the payload?**

- **Yes — provider includes a tenant identifier in the payload** (e.g. Stripe `account` field, GitHub `installation.id`) — the handler reads it and routes accordingly. Capture the exact field path.
- **Yes — but routing depends on the provider's account, which is global** — one Source per upstream account; capture how many you need to provision.
- **No — webhooks are global to your service** — single Source, single Endpoint, no per-tenant logic.

## 3. Fanout

> **How many internal services need each verified webhook? One, or several?**

- **One service** — single Endpoint under the Source. Simplest.
- **Several services, same payload** — fanout: one Endpoint per consuming service. List them.
- **Several services, each wants a subset** — fanout with per-Endpoint **filters**. Capture the filter criteria per Endpoint.
- **Several services, each wants a different shape** — fanout with per-Endpoint **transformations**. Note the constraints below.

### Transformation constraints

Transformations run as a thin JS sandbox on Svix. They're appropriate when the shape adapter is:
- stateless (no DB, no external calls);
- short (<~30 lines, no deep branching);
- per-Endpoint (not shared);
- not security-sensitive (no auth decisions).

If any of those don't hold, pull the logic into the handler instead. Plan should explicitly say which transformations are on Svix vs in handler code.

## 4. Handler shape

> **What language / framework is the consuming handler in?**

- TypeScript / JavaScript, Python, Go, Rust, Java, Kotlin, Ruby, C#, PHP — pick the matching SDK; signature verification is one call.
- Other — fall back to <https://docs.svix.com/receiving/verifying-payloads/how-manual>. Don't hand-roll HMAC otherwise.

### Follow-up — raw body access

> **Does your framework give you access to the raw request body before JSON parsing?**

- **Yes** (Express with `express.raw`, FastAPI with `Request.body()`, Go `io.ReadAll(r.Body)`, etc.) — good.
- **Unsure / no** — flag as a blocker. Verification breaks the moment a middleware re-stringifies the body. Plan must call out the exact middleware-ordering requirement.

## 5. Dedupe

> **Is the consuming handler idempotent on retry, or do we need a dedupe layer?**

- **Already idempotent** (upsert by domain id, e.g.) — great. Note it in the plan.
- **Needs dedupe** — insert `svix-id` into a "processed" table with a unique constraint; on duplicate, return 2xx without re-processing. Plan must specify which table / store.
- **Don't know yet** — flag. At-least-once delivery without dedupe means duplicate side effects.

## 6. Secrets

> **Where will the per-Endpoint `whsec_…` secrets live?**

- One secret per Endpoint (each Endpoint Svix creates has its own). Capture: secret manager, env var naming convention (e.g. `SVIX_ENDPOINT_SECRET_{ENDPOINT_NAME}`), rotation policy.
- Plan must distinguish: provider-side secret (lives on the Source, configured once when creating the Source) vs Endpoint secret (used by your handler to verify the Svix fanout). These are different things.

## 7. Operational webhooks

> **Do you want to be notified when an internal Endpoint of yours starts failing or gets auto-disabled?**

- **Yes** — same as Dispatch: subscribe `message.attempt.failing`, `endpoint.disabled`. Plan specifies destination.
- **Skip** — accepted only if internal Endpoints are monitored some other way (e.g. centralized error tracking).

## Local development

Recommend `svix listen` for testing inbound webhooks against a developer's laptop: it exposes a public URL that forwards to a local port, so the provider (or the Svix Source) can send to the listen URL during dev.

## Output

When this branch finishes, you should be able to fill out every Ingest field in <plan-template.md>. If any required field is still blank, ask one more targeted question before writing the plan.
