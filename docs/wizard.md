# Demo Wizard

## Command

`greentic-operator demo wizard`

## Behavior

- Builds a deterministic plan first.
- Default mode is dry-run (plan-only).
- Executes only with `--execute` and interactive confirmation.
- Reuses demo allow lifecycle semantics:
  - write gmap rules
  - rerun resolver
  - copy resolved manifest for demo start

## Inputs

- `--bundle <DIR>` target bundle path (must not exist on execute)
- `--mode <create|update|remove>` (default: `create`)
- `--catalog-pack <ID>` repeatable catalog ids
- `--catalog-file <PATH>` optional catalog file (JSON/YAML)
- `--pack-ref <REF>` repeatable custom refs (`oci://`, `repo://`, `store://`)
- `--tenant <TENANT>` tenant for allow rules (default: `demo`)
- `--team <TEAM>` optional team scope
- `--target <tenant[:team]>` repeatable tenant/team targets (overrides single tenant/team pair)
- `--allow <PACK[/FLOW[/NODE]]>` repeatable allow paths
- `--execute` execute instead of dry-run
- `--offline` resolve packs from cache only
- `--run-setup` run existing setup flows after execute
- `--setup-input <PATH>` optional setup answers passed to setup runner

## Catalog + custom refs

Catalog returns references only. Fetching happens in execution through distributor-client.

Catalog source options:
- built-in static list (default)
- custom file via `--catalog-file`
- custom file via `GREENTIC_OPERATOR_WIZARD_CATALOG`

## Pack resolution

Pack refs are resolved through `greentic-distributor-client` pack-fetch API.

- `oci://...` resolved directly.
- `repo://...` and `store://...` are mapped to OCI references using:
  - `GREENTIC_REPO_REGISTRY_BASE`
  - `GREENTIC_STORE_REGISTRY_BASE`

Fetched packs are copied into `bundle/packs/*.gtpack` with deterministic names.

## Setup invocation

Wizard plan includes `ApplyPackSetup` as a high-level step.

When `--run-setup` is enabled, wizard:
- collects answers per selected pack using each pack’s declared setup spec (`collect_setup_answers`)
- builds a preloaded answers map keyed by selected `pack_id`
- invokes existing setup machinery (`run_domain_command` with `DomainAction::Setup`) for messaging/events/secrets domains, filtered to selected providers only

`remove` mode skips setup execution.

## Tenants/teams/allow model

Wizard uses existing tenant/team + gmap primitives only.

No new allow storage format is introduced.
