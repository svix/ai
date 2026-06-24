# Svix Agent Skills Evals

These JSONL cases describe expected behavior for agents using the Svix plugin
Skills across Codex, Claude, Copilot, and similar harnesses.

The plugin is the installable package for an agent workspace. Each Skill is one
focused `SKILL.md` workflow inside that plugin. The evals cover webhook dispatch,
incoming webhook receivers, and integration planning while preserving the same
privacy boundary as the Skills.

Do not emit prompts, source files, webhook payloads, API keys, signing secrets,
tenant identifiers, tool arguments, or model outputs as telemetry.

## Local Validation

```bash
while IFS= read -r line; do
  printf '%s\n' "$line" | jq -e . >/dev/null
done < evals/svix-agent-skills/cases.jsonl
```

## Telvine Packaging

If this plugin is published through Telvine, publish the repository as the
plugin and treat the Svix Skills as plugin components:

```bash
npm i -g telvine
telvine login
telvine publish .
```
