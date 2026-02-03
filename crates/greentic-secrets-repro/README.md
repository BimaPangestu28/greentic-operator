# greentic-secrets-repro

Minimal reproducible crate that proves a dev secrets store can be seeded, re-opened, and read through the same API used by `greentic-operator`.

## Running the repro

```sh
RUST_LOG=greentic_secrets_repro=debug cargo test -p greentic-secrets-repro -- --nocapture
```

The `--nocapture` flag keeps the tracing logs visible and `RUST_LOG` enables the demo filter that prints the canonical URI, backend selection, and missing-secret diagnostics.

## What it proves

1. A `DevStore` can be created in a temp directory and is automatically backed by the dev provider backend.
2. Canonical URIs such as `secrets://demo/3point/core/messaging-telegram/telegram_bot_token` are resolved, seeded, and retrieved.
3. Re-opening the store at the same path returns persisted secrets.
4. Attempting to get a non-existent canonical URI yields a `NotFound` error that contains the canonical key.
