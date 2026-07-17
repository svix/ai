# Planning

Produces a written integration plan: tenancy, event type catalog, App Portal embedding, or migrating off an existing webhook system. Investigate the repo, ask only what the repo can't answer, then write the plan.

**You are only here because the user explicitly asked for a plan, a design, or an approach.** If they asked for a Svix integration and said nothing about planning, you're in the wrong file — go back to the routing table in `SKILL.md` and build it. The size of the integration is not a reason to be here.

## Plan-first contract

**This is a planning pass. Don't write code, install SDKs, or scaffold files until the plan has been produced and confirmed by the user.**

This holds for the whole planning pass, not just the first reply. The user asked what to build, not for it to be built — producing a plan *and* the code it describes means they can no longer say no to the design cheaply, which is the entire point of planning. If the user approves the plan and asks to build it, that's a new pass: re-enter through the routing table in `SKILL.md` and read <quickstart.md>, <dispatch.md>, or <ingest.md> for the implementation.

## Workflow

1. **Investigate the repo first.** Before asking anything, read the codebase. Pre-fill answers wherever you can, so the user confirms a suggestion instead of answering blind. See <triage.md> for the specific things to look for.
2. **Branch.** Default to Dispatch (sending webhooks to the user's customers) unless the prompt or repo clearly indicates Ingest. See <triage.md>.
3. **Ask only what's still unknown.** Use <dispatch-questions.md> or <ingest-questions.md>. Skip any question the repo already answers.
4. **Write the plan.** Use <plan-template.md>. Quote the user's words; don't invent details.
5. **Wrap with concrete next steps.** Detect whether Svix is already wired in the repo (see <triage.md> Step 1). If it isn't, the plan's Next Steps must include API key, SDK install, and creating the first Application — the sequence in <quickstart.md>, summarized in the plan rather than executed. If it is, skip those and go straight to the new customizations (Event Types, App Portal session URL, Ingest Sources, migration, etc.). Reference live docs (`docs.svix.com/<page>.md`) for SDK syntax in any language — don't guess.

## Rules

- **Pre-fill, don't interrogate.** This should feel like magic. If the repo has an `organization_id` column and existing event names, propose them — don't make the user type them.
- **One topic per `AskUserQuestion`, with a plain-language explainer.** Assume the user has never read the Svix docs. Avoid jargon like "tenancy model" or "taxonomy" — say "how customers map to Applications" or "list of event names."
- **Quote the user's words.** When they say "we call them organizations," use `organization_id` in the plan.
- **Stay in scope for "first working integration."** Channels, idempotency keys, operational webhooks, secret-manager choice, environment separation — these are out of scope. List them under **out of scope / TODO** in the plan so the user knows they're deferred, not forgotten. <dispatch.md> and <ingest.md> cover them when the user is ready to build.

## When to consult live docs

Append `.md` to any docs URL to get the markdown version. Read these before producing the final plan:

- <https://docs.svix.com/quickstart> — per-language setup steps.
- <https://docs.svix.com/overview> — Applications, Endpoints, Event Types, Messages.
- <https://docs.svix.com/app-portal> — embedding modes and access scopes.
- <https://docs.svix.com/event-types> — naming convention (`<group>.<event>`), bulk creation.
- <https://docs.svix.com/ingest/receiving-with-ingest> — Source types and provider presets (Ingest only).
</content>
