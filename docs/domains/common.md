# Domain Contracts (Common)

This document defines the operator expectations for provider packs across all domains.
Provider packs are distributed as `.gtpack` files and are discovered by the operator
under `providers/<domain>/*.gtpack`.

## Required folder layout

```
<project>/
  providers/
    <domain>/
      <provider>.gtpack
```

## Standard lifecycle flows

Provider packs may expose the following entry flows:

- `setup_default` (required unless explicitly allowed to be missing)
- `setup_custom` (optional)
- `diagnostics` (recommended)
- `verify_*` (optional domain-specific verification flows)

If `diagnostics` or `verify_*` is missing, the operator skips it and records a warning.
If `setup_default` is missing, the operator fails unless `--allow-missing-setup` is set.

## Input payload conventions

Operator calls provide a minimal JSON payload:

```json
{
  "tenant": "TENANT_ID",
  "team": "TEAM_ID"
}
```

`team` is omitted when not provided. Domain-specific fields may be added in the future:

- `public_base_url` if available (for webhook verification flows)

## Output expectations

Operator captures run output and artifacts under:

```
state/runs/<domain>/<pack>/<flow>/<timestamp>/
  run.json
  summary.txt
  artifacts_dir
```

Provider packs should ensure flow outcomes can be reflected in the runner `RunResult`
and that failures are surfaced via non-success status.
