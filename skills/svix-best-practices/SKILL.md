---
name: svix-best-practices
description: >-
  Guides Svix integration decisions — Dispatch vs Ingest, tenancy
  (Applications, Channels, customer UIDs), idempotency, signature
  verification, App Portal, operational webhooks, and CLI usage. Use when
  building, modifying, or reviewing any Svix integration — sending
  outbound webhooks, receiving third-party webhooks, verifying signatures,
  embedding the App Portal, configuring Sources, or writing handlers.
allowed-tools: WebFetch
---

## Integration routing

| Building…                                              | Recommended approach        | Details                  |
| ------------------------------------------------------ | --------------------------- | ------------------------ |
| Sending webhooks to your customers                     | Dispatch (`message.create`) | <references/dispatch.md> |
| Receiving third-party webhooks                         | Ingest Sources              | <references/ingest.md>   |
| Multi-tenant routing within one customer               | Channels (not Event Types)  | <references/dispatch.md> |
| Embedded webhooks management for endpoints, logs, replay       | App Portal session URL      | <references/dispatch.md> |
| Monitoring your customers' endpoint health             | Operational webhooks        | <references/dispatch.md> |
| Local development against the cloud                    | `svix listen`               | <references/cli.md>      |
| Shell scripting, bulk ops, one-off provisioning        | CLI + `jq`                  | <references/cli.md>      |

Read the relevant reference file before answering any integration question or writing code.

## Key documentation

When reading live docs, ensure to add `.md` to the end of each path to get the markdown version 
When the user's request does not clearly fit a single domain above, consult:

- [Quickstart](https://docs.svix.com/quickstart) — Start here when designing any integration.
- [API reference](https://api.svix.com/docs) — Overview of Svix's API surface.
- [Verifying payloads](https://docs.svix.com/receiving/verifying-payloads/how) — Authoritative reference for handler-side verification.
