# Dispatch questions

Use these when the user is sending webhooks to their customers. Ask via `AskUserQuestion`, one topic at a time.

## Rules

- Each question has a **plain-language explainer** in the `description` field — assume the user has never read the Svix docs.
- **Pre-fill from the repo.** If you already know the answer from the codebase (an `organization_id` column, an existing event-name enum), propose it as the first option and let the user confirm.
- **One topic per `AskUserQuestion`.** Don't dump every question at once.

The goal is a working first integration — not a production-grade one. Anything advanced (Channels for sub-tenancy, idempotency keys, operational webhooks, environment separation, secret-manager configuration) goes in the plan as **out of scope / TODO**, not as a question.

## 1. Application identifier

> **What identifier should we use for your costumers?**

Description text:

> An Application in Svix represents one of your customers — messages are sent to an Application and fanned out to that customer's endpoints. Svix lets you use your own internal IDs (UIDs) as the primary key throughout the API instead of Svix's generated `app_…` IDs, so you don't need to store a mapping.

Options — pre-fill the first one with whatever you found in the repo:

- **`<field name found in repo>`** (e.g. `organization_id`) — recommended; it's already the stable tenant key.
- **A different field** — if the user wants something else; capture exactly what.

Skip this question entirely if the repo only has one obvious candidate and no ambiguity.

## 2. Event types

> **Do you already know the types of events you'll be sending?**

Description text:

> Each Svix message has an event type like `invoice.paid` or `user.created`. Customers subscribe to the event types they care about. Svix recommends a `<group>.<event>` naming convention so the App Portal can group them in the subscription UI.

Options:

- **Yes — here they are** (pre-fill with anything you grepped out of the repo). The plan pre-creates these event types in Svix.
- **No, design together** — capture the convention in the plan; defer the list to implementation.

## 3. Customer-facing UI

> **How will your customers add their webhook URLs?**

Description text:

> Svix has a fully white-label embedded UI called the **App Portal** where your customers add endpoints, inspect delivery logs, and replay failed messages. They don't need a Svix account. Your backend generates a short-lived session URL per customer; you embed or link to it. The alternative is building your own UI on top of Svix's management API.

Options:

- **App Portal (recommended)** — fastest path; capture where in the product it'll live.
- **My own UI** — your backend wraps `endpoint.create / update / delete`. The plan must include a server-side proxy; `SVIX_AUTH_TOKEN` is never exposed to the browser.
- **No customer-facing UI yet** — endpoints provisioned by support via dashboard or CLI. Capture this as a temporary state.

## 4. Migration (only if existing webhook code is in the repo)

If Step 1 of triage didn't find an existing webhook sender, skip this question.

> **Is this replacing your existing webhook system, or running alongside it?**

Description text:

> Two webhook senders for the same customer can collide — duplicate deliveries, conflicting signatures, two sets of endpoints to manage. The plan needs to cover how the cutover works.

Options:

- **Replacing it** — capture: dual-write / shadow / hard cutover, who notifies subscribers, and that signature shape will change (customers will need to re-verify).
- **Running alongside** — capture which events go through which system and how overlap is prevented.

## Output

Fill in the Dispatch section of <plan-template.md>. List anything advanced (idempotency strategy, Channels, operational webhooks, environment separation, secrets manager choice) under **out of scope / TODO**.
