---
name: svix-integration-plan
description: >-
  Produces a written integration plan for a complete Svix integration with multi-tenant routing, event type catalog, App Portal embedding, migrating
  off an existing webhook system. Investigates the repo, asks only the
  questions Claude cannot answer on its own, then writes the plan. Use when
  the user asks "how should I integrate Svix into <this codebase>?" or wants
  a design pass before code. For "I just want to send my first webhook,"
  use the svix-quickstart skill instead.
allowed-tools: WebFetch(domain:docs.svix.com), WebFetch(domain:svix.com), WebFetch(domain:github.com), WebFetch(domain:raw.githubusercontent.com), AskUserQuestion, Read(./**), Read(~/.claude/skills/svix-integration-plan/**), Read(/.claude/skills/svix-integration-plan/**), Grep(./**), Grep(~/.claude/skills/svix-integration-plan/**), Grep(/.claude/skills/svix-integration-plan/**), Glob
---

## Workflow

This skill is plan-first. Don't write code, install SDKs, or scaffold files until the plan has been produced and confirmed.

1. **Investigate the repo first.** Before asking anything, read the codebase. Pre-fill answers wherever you can, so the user confirms a suggestion instead of answering blind. See <references/triage.md> for the specific things to look for.
2. **Branch.** Default to Dispatch (sending webhooks to the user's customers) unless the prompt or repo clearly indicates Ingest. See <references/triage.md>.
3. **Ask only what's still unknown.** Use <references/dispatch-questions.md> or <references/ingest-questions.md>. Skip any question the repo already answers.
4. **Write the plan.** Use <references/plan-template.md>. Quote the user's words; don't invent details.
5. **Wrap with concrete next steps.** Detect whether Svix is already wired in the repo (see <references/triage.md> Step 1). If it isn't, the plan's Next Steps must include API key, SDK install, and creating the first Application. If it is, skip those and go straight to the new customizations (Event Types, App Portal session URL, Ingest Sources, migration, etc.). Reference live docs (`docs.svix.com/<page>.md`) for SDK syntax in any language — don't guess.

## Rules

- **Pre-fill, don't interrogate.** This should feel like magic. If the repo has an `organization_id` column and existing event names, propose them — don't make the user type them.
- **One topic per `AskUserQuestion`, with a plain-language explainer.** Assume the user has never read the Svix docs. Avoid jargon like "tenancy model" or "taxonomy" — say "how customers map to Applications" or "list of event names."
- **Quote the user's words.** When they say "we call them organizations," use `organization_id` in the plan.
- **Stay in scope for "first working integration."** Channels, idempotency keys, operational webhooks, secret-manager choice, environment separation — these are out of scope. List them under **out of scope / TODO** in the plan so the user knows they're deferred, not forgotten.

## When to consult live docs

Append `.md` to any docs URL to get the markdown version. Read these before producing the final plan:

- <https://docs.svix.com/quickstart> — per-language setup steps.
- <https://docs.svix.com/overview> — Applications, Endpoints, Event Types, Messages.
- <https://docs.svix.com/app-portal> — embedding modes and access scopes.
- <https://docs.svix.com/event-types> — naming convention (`<group>.<event>`), bulk creation.
- <https://docs.svix.com/ingest/receiving-with-ingest> — Source types and provider presets (Ingest only).
