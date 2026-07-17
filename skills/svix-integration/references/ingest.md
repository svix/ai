# Ingest

Inbound webhooks: a Source exposes a public URL that verifies the provider's signature at the edge, fans out to one or more Endpoints, and re-signs each delivery with Svix headers. Your handler verifies the **Svix** signature on the fanout — regardless of provider.

Live docs are the source of truth — see "Reference URLs" at the bottom.

## Source Types

Source Types are presets that auto-configure Ingest for a specific provider's signature scheme.

- **Always use the preset when one exists.** Never manually wire HMAC for a provider Svix already supports. Grep `IngestSourceIn` in the user's installed SDK (or <https://github.com/svix/svix-webhooks>) to find current `type` literals and their `config:` shape
- **Don't double-verify in your handler.** Once a preset verifies the provider's signature at the edge, your handler verifies only the **Svix** signature on the fanout. Belt-and-suspenders is fine if deliberate, but the default is to trust the edge.

### Picking when there's no preset

| Question | Answer |
|----------|--------|
| Is there a preset? | Use the preset |
| Is the upstream one of your own services? | Use `svix` — generate the secret in Svix, deploy it to the sender |
| No signing at all? | `generic-webhook` — the URL is now public-unauthenticated; treat messages as untrusted |

**Don't confuse `generic-webhook` with `svix`.** `generic-webhook` accepts everything; `svix` verifies an HMAC with a `whsec_…` secret. `svix` is for upstreams you control; `generic-webhook` is the last resort.

## Handler shape

1. Receive the **raw** body — don't parse JSON before verification (re-stringification breaks the signature).
2. Verify with the official `svix` library, passing `svix-id` / `svix-timestamp` / `svix-signature` and the Endpoint's `whsec_…` secret.
3. Branch on the provider's payload (passed through verbatim).
4. Return 2xx within 15s; heavy work goes asynchronous.


### Dedupe on `svix-id`

The `svix-id` header repeats only when Svix retries the same fanout. Insert into a "processed" table with a unique constraint; on duplicate, return 2xx without re-processing.

### Verification traps

- **Wrong secret.** The **Source** secret is the **provider's** secret used at the edge. The **handler** uses the **Endpoint's** `whsec_…`. Different things, different env vars.
- **Body parsed before verification.** Use raw-body access (`express.raw`, `await request.text()`, `request.body.read`).
- **Secret missing `whsec_` prefix.** Usually a paste of `SVIX_AUTH_TOKEN` instead.
- **Replaying old captured payloads with curl.** Default tolerance is 5 minutes; use the dashboard's **Resend**.
- **Reverse proxy stripping `svix-*` headers.** Check proxy config before suspecting Svix.
- **No official `svix` library for your language?** Follow <https://docs.svix.com/receiving/verifying-payloads/how-manual>. Don't invent your own HMAC.

## Routing: fanout, filters, transformations

Filters and transformations live on **Endpoints**, not on the Source. Each consumer of the same Source gets its own view. Iterate scripts in the dashboard's **transformation playground** before saving.

| Situation | Pattern |
|-----------|---------|
| One service, all events | Single Endpoint, no filter |
| Multiple services, same events | Fanout — one Endpoint per service |
| Endpoint only cares about a subset | Filter on that Endpoint |
| Service needs a different payload shape | Transformation on that Endpoint |
| Per-tenant state-dependent logic | Skip filter/transform — do it in your handler |
| "Route A here, B there" | Two Endpoints, each with a selecting filter |

- **Filter on the Endpoint, not in your handler.** A 200 from your handler isn't ambiguous about whether you handled or ignored the event. Filtering at the Endpoint means Svix never delivers in the first place.
- **There's no built-in conditional routing inside one Endpoint.** Every Endpoint sees every event (optionally filtered). For conditional routing, create one Endpoint per route.
- **Don't transform on Svix when:** it needs database lookups or external calls, you want it unit-tested, it depends on per-tenant state, it needs auditable change tracking, or it's grown past ~30 lines with branching. Pull it into your handler. Transformations are for thin, stable, per-Endpoint shape adapters.

## Inspect and replay

- **Dashboard Resend** is the fastest replay. Ingest exposes the same Message / Message Attempt API as Dispatch — internally a fanout is a Message under an Application.
- **From the terminal:** `svix message-attempt resend`. See [cli.md](cli.md) for bulk jq patterns.
- **Wire operational webhooks** for Ingest too — don't poll for failures. `endpoint.disabled` and `message.attempt.failing` go to Slack/PagerDuty.

### Triage flow

| Symptom | Look at |
|---------|---------|
| Provider says it sent it, Svix has no record | Provider config (URL, signing secret) + Svix's source-type 401 logs |
| Source receives but Endpoint doesn't | Ingest → Logs in the dashboard — fanout entry shows your Endpoint's response (404 wrong path, 500 crash, timeout) |
| Endpoint receives but verification fails | See "Verification traps" above |
| Fanout 2xx but your DB didn't change | Your handler (order of operations, exception handling) |
| Some events succeed, some fail | Diff failed Attempt bodies against successful ones |

Wrong provider-side signing secret on the Source is the #1 cause of nothing arriving.

## Reference URLs

| Topic | URL |
|-------|-----|
| Receiving with Ingest | <https://docs.svix.com/ingest/receiving-with-ingest> |
| Transformations / filters (JS sandbox) | <https://docs.svix.com/ingest/transformations> |
| Source errors | <https://docs.svix.com/ingest/source-errors> |
| Supported providers | <https://www.svix.com/ingest/> |
| Verifying webhooks (official lib) | <https://docs.svix.com/receiving/verifying-payloads/how> |
| Verifying webhooks (manual fallback) | <https://docs.svix.com/receiving/verifying-payloads/how-manual> |
| Retries | <https://docs.svix.com/retries> |
| Idempotency (handler side) | <https://docs.svix.com/idempotency> |
| Static IPs | <https://docs.svix.com/security#ip-allow-list> |
| Interactive API docs | <https://api.svix.com/docs> |
