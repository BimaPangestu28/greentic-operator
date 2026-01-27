# Messaging Domain Contract

Provider packs for messaging must live under:

```
providers/messaging/*.gtpack
```

## Required/optional flows

- `setup_default` (required unless explicitly allowed to be missing)
- `verify_webhooks` (optional but recommended)
- `diagnostics` (recommended)
- `rotate_credentials` (optional)

## Input payload

Messaging flows receive the common payload:

```json
{
  "tenant": "TENANT_ID",
  "team": "TEAM_ID",
  "public_base_url": "https://example.ngrok.app"
}
```

`public_base_url` is recommended when webhooks are used. It is omitted if the operator
does not have a public base URL.

## Output expectations

Messaging flows should report success/failure through the runner `RunResult` so the
operator can surface diagnostics in `state/runs/`.

## Artifacts

Every demo run writes `run.json`, `summary.txt`, and the `artifacts_dir` link under `state/runs/messaging/<pack>/<flow>/<timestamp>/` so the operator UI or human reader can inspect how the pack executed.
