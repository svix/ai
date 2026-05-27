# Plan template

Produce the plan in Markdown. Quote the user's answers verbatim where they map to a decision — don't invent specifics they didn't give. If a section doesn't apply, **omit it**; don't fill it with placeholders.

## Delivery: chat or file?

Before writing the plan, ask once via `AskUserQuestion`:

> **Where should I put the plan?**

- **In chat only** — fastest; good for quick prototypes or when the plan will be discussed and revised.
- **Write to a file** — recommended for non-trivial integrations or work that spans multiple sessions. Default path: `SVIX_PLAN.md` at the repo root.
- **Both** — print in chat and write to disk.

If writing to disk: confirm the path and warn if the file already exists — don't overwrite without consent.

## Section order

```markdown
# Svix integration plan — <user's project name>

## Direction
<Dispatch | Ingest | Both>. One sentence on what each side does in this product.

## Language / SDK
<language>, `<install command>`. Quickstart: <link to the per-language quickstart>.

## Dispatch (omit if Ingest-only)

### Application identifier
- **UID field:** `<field>` from `<source — DB column / JWT claim>`.
- **Why:** <quote the user, e.g. "stable across renames, unique per tenant">.

### Event types
- **Naming convention:** `<group>.<event>`.
- **Known event types:** <list, or "TBD — to be added during implementation">.

### Customer-facing UI
- **Choice:** <App Portal embedded | App Portal session URL link | own UI on the management API | none yet>.
- **Where it lives in the product:** `<route or page>`.
- **Session URL endpoint** (if App Portal): `<backend route>`.

### Migration (omit if greenfield)
- **Existing system:** <what's being replaced>.
- **Cutover plan:** <dual-write | shadow | hard cutover>.
- **Subscriber comms:** <who notifies, when, signature-change implications>.

## Ingest (omit if Dispatch-only)

### Providers and Source Types
| Provider | Source Type | Notes |
|----------|-------------|-------|
| <name>   | <preset / `svix` / `generic-webhook`> | <signing scheme, special config> |

### Sources to provision
- <Source name>, type `<type>`.

### Fanout
For each Source:
- **Endpoint:** `<name>` → `<consuming service URL>`. Filter: `<criteria or "none">`. Transformation: `<short description or "none, handle in code">`.

### Handler
- **Framework:** <express / FastAPI / fiber / …>.
- **Raw body access:** `<how it's obtained>`.
- **Dedupe:** `<already-idempotent | svix-id into <table> with unique constraint>`.

## Out of scope / TODO

Decisions that aren't part of this plan but the user will need before going to production. Examples:
- Idempotency-Key strategy for outbound messages.
- Channels for sub-tenant routing within a customer.
- Operational webhooks (`endpoint.disabled`, `message.attempt.failing`).
- Environment separation (prod / staging Svix environments).
- Secret-manager wiring for `SVIX_AUTH_TOKEN` and per-Endpoint `whsec_…`.
- Event Type JSONSchemas.

## Next steps

If the repo doesn't have Svix wired up yet, include these setup steps first:
1. (User) Generate an API token at <https://dashboard.svix.com/api-access> and set `SVIX_AUTH_TOKEN`.
2. Install the SDK for <language>. Exact install command per language: <https://docs.svix.com/quickstart.md#install-the-svix-sdk-optional>.
3. Create the first Application, keyed by the UID field above.

Then the customization steps from this plan (drop any that don't apply):
4. (Dispatch) Pre-create the event types listed in the Event types section.
5. (Dispatch) If using the App Portal: add the `<backend route>` that mints session URLs via `svix.authentication.appPortalAccess`.
6. (Ingest) Provision the Sources listed above; share each Source URL with the corresponding provider.
7. (Migration) Execute the cutover plan from the Migration section.
```

## Rules for filling it out

- **Don't pad.** A short plan with concrete decisions beats a long one with placeholders. Omit sections that don't apply.
- **Cite the user.** If a decision came from a specific answer, say so inline: "Per the user: 'we'll use organization_id'."
- **Surface blockers.** If something stayed `TBD` that must be decided before code (raw body access for Ingest, which event types exist for Dispatch), list it under **out of scope / TODO** with a "must resolve before implementation" note.
