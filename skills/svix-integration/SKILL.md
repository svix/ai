---
name: svix-integration
description: >-
  Everything for working with Svix webhooks: first-time setup (API key,
  SDK install, first message), Dispatch (sending webhooks to your
  customers), Ingest (receiving third-party webhooks), Applications,
  Channels, customer UIDs, idempotency, App Portal embedding,
  operational webhooks, the Svix CLI, and — only when the user
  explicitly asks for one — a written integration plan (repo
  investigation, tenancy design, event type catalog, migrating off an
  existing webhook system). Use whenever you add, review, modify, or
  debug a Svix integration. For a robust webhook receiver that isn't
  Svix-specific, use receiving-webhooks.
allowed-tools: Read(${CLAUDE_SKILL_DIR}/references/**), Grep(${CLAUDE_SKILL_DIR}/references/**), Read(./**), Grep(./**), Glob, AskUserQuestion, WebFetch(domain:docs.svix.com), WebFetch(domain:api.svix.com), WebFetch(domain:www.svix.com), WebFetch(domain:svix.com), WebFetch(domain:github.com), WebFetch(domain:raw.githubusercontent.com)
---

## Mode

**Default to building.** Use the routing table below and write the code. A request to integrate Svix is a request for working code.

**Plan only when the user explicitly asks for one.** "Write me a plan", "how should I approach this?", "design this before we code", "what's the architecture?" — that, and nothing weaker, sends you to <references/planning.md>, which is plan-first: no code until the plan is confirmed.

Size is not a trigger. Multi-tenant routing, an event type catalog, App Portal embedding, a migration — build them. Don't answer a request for code with a document, and don't stop to ask whether they'd like a plan first; if they wanted one they'd have said so.

One thing to say out loud while you build, without pausing for permission: **replacing an existing webhook sender changes the signature your customers verify against.** Their handlers break on cutover unless they re-verify with Svix's scheme. Flag that in your summary, note it in the code where the cutover happens, and keep going — see the migration guidance in <references/dispatch-questions.md> for what a cutover has to cover.

## Integration routing (Building)

**First, check whether Svix is already wired into the repo.** Look for a Svix SDK in the language's manifest (`package.json`, `pyproject.toml`, `go.mod`, `Gemfile`, …) and for `SVIX_AUTH_TOKEN` in config or env files. If it isn't there, start with <references/quickstart.md> — the setup steps come before anything below. If it is, skip the quickstart; that work is done.

| Building…                                                | Recommended approach        | Details                    |
| -------------------------------------------------------- | --------------------------- | -------------------------- |
| Adding Svix to a project for the first time              | Quickstart setup path       | <references/quickstart.md> |
| Sending webhooks to your customers                       | Dispatch (`message.create`) | <references/dispatch.md>   |
| Receiving third-party webhooks                           | Ingest Sources              | <references/ingest.md>     |
| Multi-tenant routing within one customer                 | Channels (not Event Types)  | <references/dispatch.md>   |
| Embedded webhooks management for endpoints, logs, replay | App Portal session URL      | <references/dispatch.md>   |
| Monitoring your customers' endpoint health               | Operational webhooks        | <references/dispatch.md>   |
| Local development against the cloud                      | `svix listen`               | <references/cli.md>        |
| Shell scripting, bulk ops, one-off provisioning          | CLI + `jq`                  | <references/cli.md>        |

Read the relevant reference file before answering any integration question or writing code.

## Reference map

Building (the default): the routing table above.

Planning (only on an explicit request): <references/planning.md> drives, pulling in <references/triage.md> (what to look for in the repo), then <references/dispatch-questions.md> or <references/ingest-questions.md> (what to ask), then <references/plan-template.md> (what to write).

## Key documentation

When reading live docs, ensure to add `.md` to the end of each path to get the markdown version
When the user's request does not clearly fit a single domain above, consult:

- [Quickstart](https://docs.svix.com/quickstart) — Start here when designing any integration.
- [API reference](https://api.svix.com/docs) — Overview of Svix's API surface.
- [Verifying payloads](https://docs.svix.com/receiving/verifying-payloads/how) — Authoritative reference for handler-side verification.
</content>
