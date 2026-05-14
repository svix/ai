# Triage questions

Ask these first, in order. Each one branches the rest of the interview — don't skip ahead. Use `AskUserQuestion`; the bullets below are the options to present.

## 1. Direction

> **Do you send webhooks to your customers or receive them from third parties?**

- **Send (Dispatch)** — your service notifies your customers' systems about events that happened in your product.
- **Receive (Ingest)** — your service consumes webhooks from providers like Stripe, GitHub, Shopify, or your own upstream services.

> Branch to <dispatch-questions.md> or <ingest-questions.md>

## 2. Tenancy model

> **In Svix, one Application usually maps 1:1 to one of your customers (or partners). Does that fit how you think about tenants?**

- **One Application per customer** — the standard SaaS model (Stripe, Clerk). Each customer manages their own Endpoints in the App Portal.
- **One Application per partner / integration** — partner ecosystem (Shopify-style apps, marketplace extensions). Each partner is an Application; one of your domain events fans out to every enabled partner.
- **Hybrid** — both. Some Applications represent customers, others represent partners. We'll need to tag each Application by kind.
- **Something else** — describe what a "tenant" means in your system; we'll map it.

Skip this question entirely if direction is Receive-only — Ingest tenancy works differently (Sources, not Applications).

## 3. Stable identifier (UID)

> **Svix supports user-provided IDs (UIDs) you can use as the primary identifier throughout the API instead of the Svix-generated `app_…` IDs. What's the most stable identifier in your system for a tenant — `organization_id`, `customer_id`, `workspace_id`?**

- **`organization_id`** (or whatever the user calls their top-level tenant).
- **`customer_id` / `account_id`** — explicitly billing-facing.
- **Composite** — e.g. `tenant-{region}-{org_id}`. Flag that the UID must be immutable; if any component can change, it can't be the UID.
- **We don't have one yet** — recommend introducing one before integrating. UIDs are the single biggest ergonomic win in Svix; without them, every call requires looking up the Svix-generated ID first.

> Capture the exact field name and where it lives (DB column, JWT claim, etc.) — the plan will reference it by name.

## 4. Environments

> **Do you need to separate Svix data by environment (prod / staging / dev / per-developer)?**

- **Separate Svix environments** — production gets its own Svix environment; staging/dev share a non-production one. Recommended default.
- **One environment, prefixed UIDs** — `prod_org_123` vs `staging_org_123`. Workable but messy; use only if Svix environments aren't an option.
- **No separation needed** — only viable for early prototypes.

## 5. Language / SDK

> **Which language is the integration in?**

- **TypeScript / JavaScript** — `npm install svix` (camelCase APIs).
- **Python** — `pip install svix` (snake_case, `request={...}` kwargs).
- **Go / Rust / Java / Kotlin / Ruby / C# / PHP** — official SDK exists; confirm the version.
- **Other** — fall back to raw HTTP + the manual verification guide; flag that signature verification must be hand-rolled per <https://docs.svix.com/receiving/verifying-payloads/how-manual>.

> Lock this in early — it changes every code snippet in the plan and determines which Quickstart page to follow.

## After triage

Move to the relevant branch file. Don't re-ask anything covered above.
