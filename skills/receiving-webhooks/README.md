# receiving-webhooks

An [agent skill](https://agentskills.io) with provider-agnostic guidelines for
building a robust webhook receiver.
The agent loads it whenever it writes, reviews, or debugs a handler that consumes
incoming webhooks. From [Svix](https://www.svix.com), [Standard Webhooks](https://www.standardwebhooks.com/) or any other provider.

See [`SKILL.md`](./SKILL.md) for the full instructions.

## Install

```bash
npx skills add svix/ai --skill receiving-webhooks
```
