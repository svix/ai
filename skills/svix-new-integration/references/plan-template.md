# Plan template

Produce the plan in Markdown. Quote the user's answers verbatim where they map to a decision — never invent specifics they didn't give. If a section doesn't apply to this integration, **omit it**; don't fill it with placeholders.

## Delivery: chat or file?

Before writing the plan, ask the user (one `AskUserQuestion`):

> **Where should I put the plan?**

- **In chat only** — fastest; good for quick prototypes or when the plan will be discussed and revised inline.
- **Write to a file** — recommended when the integration is non-trivial or will be implemented over multiple sessions. Ask for the path; default suggestion: `docs/svix-integration-plan.md` or `SVIX_PLAN.md` at the repo root.
- **Both** — print in chat and write to disk.

If writing to disk: confirm the path before writing, and warn if the file already exists (don't overwrite without consent).

## Section order

```markdown
# Svix integration plan — <user's project name>

## Direction
<Dispatch | Ingest | Both>. One sentence on what each side does in this product.

## Language / SDK
<language>, `<install command>`. Quickstart: <link to the per-language Quickstart>.

## Identifiers
- **Application UID source:** `<field name>` from `<source — DB column / JWT claim>`.
- **Why this field:** <quote user's reasoning, e.g. "stable across renames, scoped per tenant">.
- **Channel naming** (if used): `<shape, e.g. project:{slug}>`.

## Environments
- Production: <Svix env name + how it's keyed>.
- Non-production: <Svix env name + how it's keyed>.
- Local dev: `<svix listen | shared dev env | self-hosted>`.

## Dispatch (omit if Ingest-only)

### Tenancy
- **One Application per:** <customer | partner | both>.
- **Provisioning trigger:** <on signup | on first webhook enable | manual>.

### Event taxonomy
- **Naming convention:** `<group>.<event>` (period-delimited).
- **Known event types:** <list, or "TBD — to be added during implementation">.
- **Schemas:** <Draft 7 JSONSchema | none yet>.

### Idempotency
- **Source of stable identity:** <outbox row id | queue message id | domain entity id + version>.
- **Idempotency-Key shape:** `<event-name>-<entity-id>-<version-or-timestamp>`, ASCII, ≤256 chars.
- **Where the call happens:** <outbox worker | queue consumer | request handler>.

### Customer-facing UI
- **App Portal:** <embedded iframe | svix-react | custom UI calling management API | none>.
- **Access scope:** <full | ViewBase read-only>.
- **Branding:** <colors / dark mode / hide-nav settings>.
- **Session URL endpoint:** `<route in the app's backend>`.

### Operational webhooks
- **Subscribed events:** <list>.
- **Destinations:** <Slack channel | PagerDuty service | audit log table>.
- **Secret env var:** `SVIX_OPERATIONAL_WHSEC` (or whatever the user named it).

### Secrets
- `SVIX_AUTH_TOKEN` lives in <secret manager>, per environment. Server-side only.

## Ingest (omit if Dispatch-only)

### Providers and Source Types
| Provider | Source Type | Notes |
|----------|-------------|-------|
| <name>   | <preset / `svix` / `generic-webhook`> | <signing scheme, special config> |

### Sources to provision
- <Source name>, type `<type>`, credentials in `<secret manager path>`.

### Fanout
For each Source:
- **Endpoint:** `<name>` → `<consuming service URL>`. Filter: `<criteria or "none">`. Transformation: `<short description or "none, handle in code">`.

### Handler
- **Framework:** <express / FastAPI / fiber / …>.
- **Raw body access:** `<how it's obtained>`.
- **Dedupe:** insert `svix-id` into `<table>` with unique constraint.
- **Secret env vars:** `<one per Endpoint, naming convention>`.

### Operational webhooks (Ingest)
- Same as above if applicable, or "skip — internal endpoints monitored via <X>".

## Migration / coexistence (omit if greenfield)
- **Existing system:** <what's being replaced>.
- **Cutover plan:** <dual-write | shadow | hard cutover>.
- **Subscriber communication:** <who notifies, when, signature change implications>.

## Out of scope / explicit TODOs
- <decisions the user deferred, with the trigger for revisiting>.

## Next steps
1. Provision the Svix environment(s) and copy `SVIX_AUTH_TOKEN` into the secret manager.
2. (Dispatch) Pre-create Event Types listed above.
3. (Ingest) Provision Sources listed above; share Source URLs with the providers.
4. Install the SDK in <language>.
5. Hand off to the `svix-best-practices` skill for implementation.
```

## Rules for filling it out

- **Don't pad.** A short plan with seven concrete decisions beats a long one with placeholders.
- **Cite the user.** If a decision came from a specific answer, say so inline: "Per the user: 'we'll use organization_id'."
- **Surface blockers.** If something stayed `TBD` that really must be decided before code (idempotency strategy, raw body access), list it under **Out of scope** with an explicit "must resolve before implementation" note.
- **Hand off cleanly.** End the plan with a one-line instruction: from here, switch to `svix-best-practices`.
