# PR-OP-01 — greentic-operator: component@0.6.0 QA-driven setup/upgrade/remove (CBOR + i18n)

Repo: `greentic-operator`

Status: completed

Note: supersedes older OP docs focused on renderer-library/cardkit. Those are historical/outdated.

## Delivered
- Implemented component QA orchestration path:
  - `qa-spec(mode)` invocation
  - i18n key export + reference validation
  - `apply-answers(mode, current_config, answers)`
  - strict config-schema validation
- Implemented canonical CBOR envelope persistence for provider config:
  - single file per provider instance: `config.envelope.cbor`
  - persisted provenance fields (`component_id`, `abi_version`, `resolved_digest`, `describe_hash`, optional `schema_hash`, `operation_id`, optional `updated_at`)
- Implemented contract cache persistence per resolved digest under `_contracts`.
- Enforced contract drift behavior with explicit override:
  - hard-fail when stored `describe_hash != resolved describe_hash`
  - diagnostic code in message: `OP_CONTRACT_DRIFT`
  - override flag: `--allow-contract-change`
- Added backup policy for envelope writes:
  - enabled via `--backup`
  - single overwrite backup file: `config.envelope.cbor.bak`
- Improved atomic writes:
  - temp write + fsync + rename + parent dir sync (best effort)
- Enforced i18n hard-fail behavior:
  - missing i18n export => `OP_I18N_EXPORT_MISSING`
  - unknown i18n key in QA spec => `OP_I18N_KEY_MISSING`

## CLI surfaces updated
- `domain setup ... --allow-contract-change --backup`
- `demo setup ... --allow-contract-change --backup`
- `demo up ... --allow-contract-change --backup` (provider setup path)

## Tests
- Added/updated focused tests for:
  - strict schema required-property failures
  - contract drift detection
  - provider setup envelope behavior and updated setup options
- Full suite run completed with `cargo test`.

## Acceptance check
- Operator can execute 0.6.0 QA setup path and persist deterministic envelope state.
- Drift and i18n failures are explicit and non-silent.
- Config writes are atomic, with optional `.bak` backup policy.
- Test suite passes.
