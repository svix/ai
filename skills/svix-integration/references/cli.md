# CLI

The [Svix CLI](https://docs.svix.com/tutorials/cli) lets you perform common actions in the Svix API as Webhook Sender (Creating endpoints, applications messages, etc) and using Svix Listen to receive webhooks in localhost.

## When to reach for it

- **One-off provisioning** — creating an Application, Endpoint, or test Message from a terminal instead of writing SDK code.
- **Shell scripting with `jq`** — bulk operations over the JSON output (e.g. resending every failed Attempt, counting Endpoints per Application).
- **Local relay** — `svix listen http://localhost:<port>/<path>` forwards a public URL to your local server. The only CLI command that doesn't need `SVIX_AUTH_TOKEN`.

## Rules

- **Don't shell out to `svix` from production code.** Use the [SDK](https://docs.svix.com/quickstart) in your language. For orchestration from Terraform, CI, or any non-shell environment, call the [API](https://api.svix.com/docs) directly.
- **Always pass `--data-uid` on `application create`.** Without it, you can only address the customer by the Svix-generated `app_…` id instead of your own stable id.
- **Endpoints must be public HTTPS.** Svix's cloud cannot deliver to `localhost` or other unroutable addresses — `svix listen` is the supported workaround for local development.
- **Treat `svix listen` URLs as ephemeral.** They change every restart — don't hardcode them. Use a real dev environment for long-lived setups.
- **Use language-specific quickstarts when porting CLI calls to code.** Python uses snake_case `request={...}`; TS/JS use camelCase. Pulling examples from the wrong language gives wrong argument shapes.
