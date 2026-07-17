# Quickstart

First-time setup: get a project sending its first webhook. API key, SDK install, Application per customer, `message.create`, subscribe endpoints.

Read this when the repo has no Svix wiring yet. If Svix is already installed, skip to [dispatch.md](dispatch.md) or [ingest.md](ingest.md) — the setup steps below are already done.

Live docs are the source of truth — see "Reference URLs" at the bottom and verify version-sensitive details (SDK signatures, package names) before writing code.

## Order of operations

1. Get an API key from the dashboard (the user does this — you can't).
2. Detect the project's language and install the matching SDK.
3. Create an **Application** per customer, keyed by your own customer ID (`uid`).
4. Send a **Message** with an event type and payload.
5. Subscribe customers to endpoints — App Portal (recommended) or the API.
6. Verify a message went out.

**Infer the SDK from the project, don't assume.** Read the repo and use that language's SDK. The examples below are JavaScript/TypeScript; the call shapes are identical across SDKs, but the argument conventions are not (Python uses snake_case `request={...}`; TS/JS use camelCase). **Confirm the exact syntax for the detected language against the live docs before writing code** — <https://docs.svix.com/quickstart.md#create-a-consumer-application> has a tab per language.

## Step 1: Get an API key

The user must create this — it can't be scripted. Have them open the [API Access page](https://dashboard.svix.com/api-access) and generate a token, then store it as an environment variable. Never hardcode it.

```bash
export SVIX_AUTH_TOKEN="testsk_..."
```

Tokens are region-specific (US, EU, India). The SDK reads the region from the token, so no base URL is needed.

`SVIX_AUTH_TOKEN` is server-side only — see [dispatch.md](dispatch.md) for what leaking it costs.

## Step 2: Install the SDK

Fetch the current install command for the detected language from <https://docs.svix.com/quickstart.md#install-the-svix-sdk-optional> — it lists the exact command per language (and the CLI), so use it as the source of truth rather than guessing the package name or version.

## Step 3: Create an application per customer

Each of your customers gets one Svix Application. Pass your own internal customer identifier as the `uid` so you can address it later without storing Svix's generated `app_…` IDs.

```ts
import { Svix } from "svix";

const svix = new Svix(process.env.SVIX_AUTH_TOKEN!);

const app = await svix.application.create({
  name: "Acme Inc",
  uid: "customer-123", // your internal customer ID
});
```

Creating an application is idempotent on `uid` — calling it again with the same `uid` returns the existing application rather than erroring.

One customer = one Application is the default model. For sub-tenancy *within* one customer, see Channels in [dispatch.md](dispatch.md).

## Step 4: Send a message

A message is one webhook event for one application. Address the application by the `uid` from Step 3. Include the event type inside the payload too, so consumers can branch on it without parsing headers.

**Infer the event types from the codebase** — grep for existing event names, enum values, or queue topics rather than inventing a taxonomy.

```ts
await svix.message.create("customer-123", {
  eventType: "invoice.paid",
  eventId: "evt_Wqb1k73rXprtTm7Qdlr38G", // optional, for idempotency
  payload: {
    type: "invoice.paid",
    id: "invoice_WF7WtCLFFtd8ubcTgboSFNql",
    status: "paid",
  },
});
```

`eventType` uses a `<group>.<event>` convention (e.g. `invoice.paid`). Setting `eventId` to a value derived from the source event makes the send idempotent — Svix dedupes repeated sends with the same `eventId`.

Once you're past the first message, set `Idempotency-Key` on every `message.create` — see [dispatch.md](dispatch.md), including the traps that silently defeat it.

## Step 5: Subscribe customers to webhooks

A message only fans out to endpoints that exist for the application. Two ways to add them:

**App Portal (recommended).** A pre-built, brandable UI where your customers add their own endpoints, inspect delivery logs, and replay failures — no Svix account needed. Generate a short-lived magic link from your backend:

```ts
const dashboard = await svix.authentication.appPortalAccess("customer-123", {});
// redirect or embed dashboard.url
```

**Endpoint API.** Create endpoints yourself when you collect the URL through your own UI. Your backend wraps `endpoint.create` — `SVIX_AUTH_TOKEN` never reaches the browser:

```ts
await svix.endpoint.create("customer-123", {
  url: "https://api.example.com/svix-webhooks/",
  description: "My main endpoint",
});
```

Endpoints must be public HTTPS — Svix's cloud can't deliver to `localhost`. Use `svix listen` for local development ([cli.md](cli.md)).

## Verify

- `svix.message.create(...)` returned without throwing.
- In the [dashboard](https://dashboard.svix.com) (or App Portal), the message appears under the application and shows a delivery attempt to each endpoint.
- For end-to-end testing without a real consumer, use `svix listen` to get a temporary public URL that relays to localhost — see [cli.md](cli.md).

## Checklist

- [ ] API key generated and set as `SVIX_AUTH_TOKEN` (not hardcoded)
- [ ] SDK installed for the project's language
- [ ] Application created with a customer `uid`
- [ ] First message sent with an `eventType` and payload
- [ ] At least one endpoint subscribed (App Portal link or `endpoint.create`)
- [ ] Message and delivery attempt visible in the dashboard / App Portal

## After the first message

The first message working is not a production integration. Next, in rough order:

- **Idempotency** on `message.create`, and the outbox pattern → [dispatch.md](dispatch.md)
- **Operational webhooks** so you learn about broken customer endpoints before they complain → [dispatch.md](dispatch.md)
- **Event Type catalog** with JSONSchemas → <https://docs.svix.com/event-types>
- **Handler-side verification**, if you also consume webhooks → the `receiving-webhooks` skill

## Reference URLs

| Topic | URL |
|-------|-----|
| Quickstart (per-language) | <https://docs.svix.com/quickstart> |
| Install commands per language | <https://docs.svix.com/quickstart.md#install-the-svix-sdk-optional> |
| Overview / concepts | <https://docs.svix.com/overview> |
| API Access (generate a token) | <https://dashboard.svix.com/api-access> |
| Event Types | <https://docs.svix.com/event-types> |
| Consumer App Portal | <https://docs.svix.com/app-portal> |
| Interactive API docs | <https://api.svix.com/docs> |
</content>
</invoke>
