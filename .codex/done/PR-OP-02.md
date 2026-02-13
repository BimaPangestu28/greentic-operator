# PR-OP-02 — greentic-operator: provider pack integration + guardrails + offline E2E fixtures

Repo: `greentic-operator`

Status: completed

Note: supersedes older OP docs focused on renderer-library/cardkit. Those are historical/outdated.

## Delivered
- Drift guardrail behavior implemented for config updates:
  - hard-fail on stored/resolved `describe_hash` mismatch by default
  - override via `--allow-contract-change`
  - diagnostic code in error message: `OP_CONTRACT_DRIFT`
- Atomic envelope writes and backup policy implemented:
  - write temp file, `fsync`, rename, parent dir sync (best effort)
  - backup only when `--backup` is passed
  - single overwrite backup path: `config.envelope.cbor.bak`
- Offline fixture registry added with required layout under:
  - `tests/fixtures/registry/index.json`
  - `tests/fixtures/registry/components/messaging-telegram/*`
- Fixture resolver added for operator tests:
  - `tests/support/fixture_resolver.rs`
- Offline E2E coverage added:
  - setup → upgrade → remove fixture-driven config sequence
  - backup overwrite semantics (`.bak` reflects prior envelope)
  - drift block + override behavior validated

## Tests added/updated
- `tests/op02_fixture_registry.rs`
  - validates required registry layout and CBOR/JSON decodability
- `tests/op02_offline_e2e.rs`
  - uses fixture resolver and validates setup/upgrade/remove + drift + backup

## Acceptance check
- Drift is detected and blocks by default.
- Drift override is available only via `--allow-contract-change`.
- Offline E2E tests run without network.
- Atomic updates and backup overwrite policy are covered by tests.
- `cargo test` passes.
