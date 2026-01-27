# Secrets Domain Contract

Provider packs for secrets must live under:

```
providers/secrets/*.gtpack
```

## Required/optional flows

- `setup_default` (required unless explicitly allowed to be missing)
- `diagnostics` (recommended)
- verify flows are handled via `doctor` validation (see OP-PR-08)

## Input payload

Secrets flows receive the common payload:

```json
{
  "tenant": "TENANT_ID",
  "team": "TEAM_ID"
}
```

## Output expectations

Secrets provider packs should surface failures through the runner `RunResult`. For
additional validation, the operator runs `greentic-pack doctor` with local validators
when available.

## Artifacts

Secrets setups also emit `run.json`/`summary.txt` under `state/runs/secrets/<pack>/<flow>/<timestamp>/`, making it easy to correlate doctor checks against what the pack reported.
