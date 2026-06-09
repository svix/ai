# Example: GitHub PR reviews

This walkthrough sets up automatic AI code review on every pull request opened
in a GitHub repository. When a PR is opened, GitHub sends the event to Svix,
Hermes polls and picks it up, fetches the diff, writes a review, and posts it
as a PR comment — all without exposing a public endpoint.

## Step 1 — Create a Svix ingest source

An ingest source is the HTTPS endpoint Svix exposes for GitHub to POST to.
Create one from the [Svix dashboard](https://dashboard.svix.com) under
**Svix Ingest → Sources → Create source**

### 1.1 Create Source UI - Step 1

1. Add the name 'github-prs' or whatever you prefer
2. Keep 'Source Type' as Webhooks

### 1.2 Create Source UI - Step 2 

1. Change from 'Generic Webhooks' to 'Github'
2. Click 'Enable Authentication'
3. Add a Secret (store it because you are going to need it later)
 
## Step 3 Create a Polling Endpoint Destination

1. Go to the Destinations Tab and Click 'Add Endpoint'
2. Change the Webhook type from 'Webhook' to 'Polling Endpoint'
3. Click 'Create'

## Step 3.1 Store your Endpoint secret

1. In your recently created Polling Endpoint, click 'Create API key'
2. In your ~/.hermes/env, paste your secret as the following environment variable
```
SVIX_GITHUB_INGEST_TOKEN=sk_endp......
```

## Step 4 — Add the GitHub webhook

Go to your repository on GitHub: **Settings → Webhooks → Add webhook**.

| Field | Value |
| --- | --- |
| Payload URL | **The ingest URL from step 1** |
| Content type | `application/json` |
| Events | **Let me select individual events** → Pull requests |

Save the webhook. GitHub will send a ping event; Svix will receive it.



## Step 5 — Configure the route

Add the following to `~/.hermes/config.yaml` under `platforms`:

```yaml
platforms:
  svix:
    enabled: true
    interim_assistant_messages: false
    tool_progress: 'off'
    extra:
      poll_interval: 5
      routes:
        github_prs:
          url: https://api.svix.com/api/v1/app/<app_id>/poller/<sink_id>/
          auth_token_env: SVIX_GITHUB_INGEST_TOKEN
          prompt: |
            PR #{number} ({action}): {pull_request.title}
            Author: {pull_request.user.login}
            Branch: {pull_request.head.ref} → {pull_request.base.ref}
            Repo: {repository.full_name}

            If the action is "closed" or "labeled", don't do anything.

            Otherwise:
            1. Run: gh pr diff {number} --repo {repository.full_name}
            2. Review the diff for correctness, security issues, and clarity.
            3. Write a concise, actionable review.

            Your response will be posted as a PR comment automatically — do not
            call gh pr comment yourself.
          deliver: github_comment
          deliver_extra:
            repo: '{repository.full_name}'
            pr_number: '{number}'
```

The `deliver: github_comment` line tells the plugin to post the agent's final
response as a PR comment via the `gh` CLI. The prompt explicitly tells the
agent not to post the comment itself, which would cause duplicates.

## Step 6 — Restart Hermes and test

```bash
hermes restart   # or: hermes gateway restart
```

Open a pull request in the configured repository. Within a few seconds Hermes
will poll the event, fetch the diff, and post a single review comment on the PR.