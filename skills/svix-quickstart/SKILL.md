---
name: svix-quickstart
description: >-
  Gets a working Svix webhook integration sending its first message as
  fast as possible — API key, SDK install, create an application, send a
  message, subscribe customers. Use when adding Svix to a project for the
  first time, or when the user wants to send their first webhook and just
  needs the setup steps, not an architecture discussion.
allowed-tools: WebFetch
---

# Svix Quickstart

Get a project sending its first webhook with Svix as fast as possible.

## When to Use

- Adding Svix to a project for the first time
- The user wants to send their first webhook and needs the setup steps
- Prototyping a webhook flow against the Svix API

## When Not to Use

- Designing a non-trivial integration (multi-tenant routing, receiving
  third-party webhooks, embedding the portal) — those need an architecture
  pass before any quickstart code

## Workflow

1. Get an API key from the dashboard (the user does this — you can't).
2. Detect the project's language and install the matching SDK.
3. Create an **Application** per customer, keyed by your own customer ID (`uid`).
4. Send a **Message** with an event type and payload.
5. Subscribe customers to endpoints — App Portal (recommended) or the API.
6. Verify a message went out.

**Infer the SDK from the project, don't assume.** Look at the repo and use that
language's SDK. The examples below are JavaScript/TypeScript; the call shapes
are identical across SDKs. **Always confirm the exact syntax for the detected
language against the live docs before writing code**
<https://docs.svix.com/quickstart.md#create-a-consumer-application> has a tab
per language (append `.md` to any docs URL for the LLM-readable version).

## Step 1: Get an API key

The user must create this — it can't be scripted. Have them open the
[API Access page](https://dashboard.svix.com/api-access) and generate a token,
then store it as an environment variable. Never hardcode it.

```bash
export SVIX_AUTH_TOKEN="testsk_..."
```

Tokens are region-specific (US, EU, India). The SDK reads the region from the
token, so no base URL is needed.

## Step 2: Install the SDK

Fetch the current install command for the detected language from
<https://docs.svix.com/quickstart.md#install-the-svix-sdk-optional> — it lists
the exact command per language (and the CLI), so use it as the source of truth
rather than guessing the package name or version.

## Step 3: Create an application per customer

Each of your customers gets one Svix Application. Pass your own internal
customer identifier as the `uid` so you can address it later without storing
Svix's generated IDs.

```ts
import { Svix } from "svix";

const svix = new Svix(process.env.SVIX_AUTH_TOKEN!);

const app = await svix.application.create({
  name: "Acme Inc",
  uid: "customer-123", // your internal customer ID
});
```

Creating an application is idempotent on `uid` — calling it again with the same
`uid` returns the existing application rather than erroring.

## Step 4: Send a message

A message is one webhook event for one application. Address the application by
the `uid` from Step 3. Include the event type inside the payload too, so
consumers can branch on it without parsing headers.

**Infer the event types from the codebase**

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

`eventType` uses a `<group>.<event>` convention (e.g. `invoice.paid`). Setting
`eventId` to a value derived from the source event makes the send idempotent —
Svix dedupes repeated sends with the same `eventId`.

## Step 5: Subscribe customers to webhooks

A message only fans out to endpoints that exist for the application. Two ways to
add them:

**App Portal (recommended).** A pre-built, brandable UI where your customers add
their own endpoints, inspect delivery logs, and replay failures — no Svix
account needed. Generate a short-lived magic link from your backend:

```ts
const dashboard = await svix.authentication.appPortalAccess("customer-123", {});
// redirect or embed dashboard.url
```

**Endpoint API.** Create endpoints yourself when you collect the URL through
your own UI:

```ts
await svix.endpoint.create("customer-123", {
  url: "https://api.example.com/svix-webhooks/",
  description: "My main endpoint",
});
```

## Verify

- `svix.message.create(...)` returned without throwing.
- In the [dashboard](https://dashboard.svix.com) (or App Portal), the message
  appears under the application and shows a delivery attempt to each endpoint.
- For end-to-end testing without a real consumer, use `svix listen` to get a
  temporary public URL that relays to localhost — see
  <https://docs.svix.com/tutorials/cli.md>.

## Next Steps

- Full per-language reference → <https://docs.svix.com/quickstart.md>
- Verifying signatures, Channels, App Portal embedding, idempotency, Ingest
  <https://docs.svix.com/overview.md>

## Checklist

- [ ] API key generated and set as `SVIX_AUTH_TOKEN` (not hardcoded)https://docs.svix.com/quickstart#install-the-svix-sdk-optional
- [ ] SDK installed for the project's language
- [ ] Application created with a customer `uid`
- [ ] First message sent with an `eventType` and payload
- [ ] At least one endpoint subscribed (App Portal link or `endpoint.create`)
- [ ] Message and delivery attempt visible in the dashboard / App Portal
