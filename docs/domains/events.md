# Events Domain Contract

Provider packs for events must live under:

```
providers/events/*.gtpack
```

## Required/optional flows

- `setup_default` (required unless explicitly allowed to be missing)
- `verify_subscriptions` (optional for now)
- `diagnostics` (recommended)

## Input payload

Events flows receive the common payload:

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

Events flows should report success/failure through the runner `RunResult` so the
operator can surface diagnostics in `state/runs/`.
