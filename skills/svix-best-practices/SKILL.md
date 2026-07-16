---
name: svix-best-practices
description: >-
  Guides Svix integration work: first-time setup (API key, SDK install,
  first message), Dispatch (sending webhooks to your customers), Ingest
  (receiving third-party webhooks), tenancy (Applications, Channels,
  customer UIDs), idempotency, App Portal embedding, operational
  webhooks, and CLI usage. Use whenever you add Svix to a project, send
  a first webhook, or write, review, or modify code that uses Svix. For
  a design pass on a non-trivial integration before any code is written,
  use svix-integration-plan. For a webhook receiver that isn't
  Svix-specific, use receiving-webhooks.
allowed-tools: Read(./**), Read(~/.claude/skills/svix-best-practices/**), Grep(./**), Grep(~/.claude/skills/svix-best-practices/**), Glob, WebFetch(domain:docs.svix.com), WebFetch(domain:api.svix.com), WebFetch(domain:www.svix.com), WebFetch(domain:github.com), WebFetch(domain:raw.githubusercontent.com)
---

## Integration routing

**First, check whether Svix is already wired into the repo.** Look for a Svix SDK in the language's manifest (`package.json`, `pyproject.toml`, `go.mod`, `Gemfile`, …) and for `SVIX_AUTH_TOKEN` in config or env files. If it isn't there, start with <references/quickstart.md> — the setup steps come before anything below. If it is, skip the quickstart; that work is done.

| Building…                                                | Recommended approach        | Details                   |
| -------------------------------------------------------- | --------------------------- | ------------------------- |
| Adding Svix to a project for the first time              | Quickstart setup path       | <references/quickstart.md> |
| Sending webhooks to your customers                       | Dispatch (`message.create`) | <references/dispatch.md>  |
| Receiving third-party webhooks                           | Ingest Sources              | <references/ingest.md>    |
| Multi-tenant routing within one customer                 | Channels (not Event Types)  | <references/dispatch.md>  |
| Embedded webhooks management for endpoints, logs, replay | App Portal session URL      | <references/dispatch.md>  |
| Monitoring your customers' endpoint health               | Operational webhooks        | <references/dispatch.md>  |
| Local development against the cloud                      | `svix listen`               | <references/cli.md>       |
| Shell scripting, bulk ops, one-off provisioning          | CLI + `jq`                  | <references/cli.md>       |

Read the relevant reference file before answering any integration question or writing code.

## Key documentation

When reading live docs, ensure to add `.md` to the end of each path to get the markdown version
When the user's request does not clearly fit a single domain above, consult:

- [Quickstart](https://docs.svix.com/quickstart) — Start here when designing any integration.
- [API reference](https://api.svix.com/docs) — Overview of Svix's API surface.
- [Verifying payloads](https://docs.svix.com/receiving/verifying-payloads/how) — Authoritative reference for handler-side verification.
</content>
