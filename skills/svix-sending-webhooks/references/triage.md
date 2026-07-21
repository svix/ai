# Triage

## Step 1 — Investigate the repo

Before asking anything, read the codebase and answer these for yourself:

- **Language / framework.** Determines which SDK to recommend. Don't ask — infer.
- **Tenant identifier.** Look for `organization_id`, `customer_id`, `workspace_id`, `tenant_id`, or similar in the DB schema, ORM models, or JWT claims. This becomes the Svix Application UID.
- **Domain events.** Grep for existing event names, enum values, queue topics, or types like `*EventType`, `*.Event`. These are candidate Svix event types.
- **Existing Svix integration.** If present, the plan's Next Steps should skip API-key / SDK-install / first-Application and focus on the new customizations.
- **Existing non-Svix webhook code.** Look for outbound HMAC signing, `crypto.createHmac`, `webhook` routes, queue consumers that POST externally. If present, this is a migration from a homegrown sender, not greenfield.
- **Customer-facing UI.** Is there a settings / integrations page where customers add URLs? If so, the user may want to either embed the App Portal there or wrap the management API.

Surface the findings when you ask each question — "I see you have an `organization_id` column; should we use that as the Application UID?" beats "what's your tenant identifier?"

## Step 2 — Direction

Most Svix integrations are **Dispatch** (sending webhooks to your customers). Default there unless the user's prompt clearly mentions receiving from a third party (Stripe, GitHub, Shopify, an internal upstream service).

**If the prompt or repo makes the direction obvious, skip the question and branch:**
- Dispatch → <dispatch-questions.md>
- Ingest → <ingest-questions.md>

**Only if it's genuinely ambiguous,** ask one question via `AskUserQuestion`:

> **What are you trying to do today — send webhooks to your customers, or receive them from a third party?**
>
> Svix supports both. We'll set up one side now; you can add the other later.

- **Send to my customers (Dispatch)** → <dispatch-questions.md>
- **Receive from a third party (Ingest)** → <ingest-questions.md>
