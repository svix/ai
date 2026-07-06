# Svix Agent Skills Evals

These JSONL cases are seed eval cases for external agent harness validation.
They are not run by Svix at runtime and are not part of this repository's
application test suite. A maintainer or downstream harness can use them when
checking whether an agent followed the Svix Skills correctly.

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
