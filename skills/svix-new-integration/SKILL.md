---
name: svix-new-integration
description: >-
  Discovery interview for a brand-new Svix integration. Ask the user a
  short set of branching questions (direction, tenancy, identifiers,
  event taxonomy, idempotency, App Portal, environments), then produce
  a written implementation plan before any code is written. Use when
  the user is starting a fresh Svix integration, doesn't yet know
  which Svix primitives to use, or asks "how should I integrate Svix
  into <this codebase>?"
allowed-tools: WebFetch, AskUserQuestion
---

## Workflow

This skill is interview-first. Do **not** write code, install SDKs, or scaffold files until the plan has been produced and confirmed.

1. **Triage** — start with <references/triage.md>. This determines the direction (Dispatch, Ingest, both) and the high-level tenancy model.
2. **Branch** — depending on the triage answers, walk the user through:
   - <references/dispatch-questions.md> if they are sending webhooks to their own customers/partners.
   - <references/ingest-questions.md> if they are receiving webhooks from third-party providers.
   - Both files, in order, if the integration is bidirectional.
3. **Write the plan** — use <references/plan-template.md>. Quote the user's answers back; do not invent details they didn't give.
4. **Confirm, then hand off** — once the plan is approved, switch to the `svix-best-practices` skill for the actual implementation. This skill's job ends at the plan.

## Interview rules

- **One topic at a time.** Use `AskUserQuestion` per topic with 2–4 concrete options plus an "Other" fallback. Don't dump every question at once — branching depends on earlier answers.
- **Quote the user's words.** When they say "we call them organizations," use `organization_id` in the plan, not a generic placeholder.
- **Surface tradeoffs, don't decide silently.** If an answer has a non-obvious downstream cost (e.g. "we'll use Channels per project" implies untyped strings, no validation), flag it in the question's description.
- **Skip questions that don't apply.** If the user only sends webhooks, never ask Ingest questions. If they have no partners, skip the partner-routing branch.
- **Resist scope creep.** A discovery interview is not the place to debate Event Type schemas in depth — capture the decision and move on. Detailed schema design happens during implementation.

## When to consult live docs

When reading live docs, ensure to add `.md` to the end of each path to get the markdown version 

Read these before answering version-sensitive questions or producing the final plan:

- <https://docs.svix.com/quickstart> — canonical setup steps per language.
- <https://docs.svix.com/common-usage-examples> — patterns: multi-tenant, multi-channel, partner ecosystem, hybrid.
- <https://docs.svix.com/overview> — Applications, Endpoints, Event Types, Messages, Channels.
- <https://docs.svix.com/event-types> — naming conventions (`<group>.<event>`), schemas, bulk creation.
- <https://docs.svix.com/channels> — when Channels beat Event Types beat separate Applications.
- <https://docs.svix.com/app-portal> — embedding modes, access scopes, branding.
- <https://docs.svix.com/idempotency> — deterministic key construction.
- <https://docs.svix.com/ingest/receiving-with-ingest> — Source types and provider presets.
