greentic-operator: MVP spec (dev + demo)
Goals

One coherent UX: greentic-operator dev ... and greentic-operator demo ...

A single project directory is the source of truth.

Operator generates resolved manifests per tenant/team so runtimes don’t “scan”.

Demo is a bundle directory that runs anywhere.

Non-goals (for now)

Roles/OAuth enforcement (but policy format anticipates it)

Production deploy targets (but design keeps the same “resolved manifest” concept)

Directory conventions
Project directory
<project>/
  greentic.yaml
  providers/
    messaging/
      messaging-telegram.gtpack
      messaging-webex.gtpack
  packs/
    pack1/                 # editable pack directory (dev)
      pack.yaml
      flows/
      components/
    pack2.gtpack           # prebuilt pack
  tenants/
    tenant1/
      tenant.gmap
      teams/
        team1/
          team.gmap
  state/
    resolved/              # generated per tenant/team manifests (dev)
    gtbind/                # optional cache (dev)
    pids/
    logs/

Demo bundle directory (output of demo build)
demo-bundle/
  greentic.demo.yaml
  providers/
    messaging/*.gtpack
  packs/
    *.gtpack
  tenants/
    .../*.gmap
  resolved/
    tenant1.team1.yaml
  state/                   # empty; runtime writes here

.gmap format (line-oriented, branch-per-line)

Syntax: <path> = <policy>

path supports:

_ (global default)

pack_id

pack_id/_

pack_id/flow_id

pack_id/flow_id/node_id

policy supports now:

public

forbidden

reserved for later:

roles:[a,b]

oauth

Matching: most-specific wins
(node > flow > pack > pack/_ > _)

Overlay: tenant then team (team overrides tenant)

Example:

_ = forbidden
demo-pack/main = public
demo-pack/_ = forbidden

Resolved manifest format (the operator-generated “truth”)

resolved/<tenant>[.<team>].yaml:

version: "1"
tenant: tenant1
team: team1
project_root: "./"                # relative in demo bundle, absolute in dev is ok too

providers:
  messaging:
    - "providers/messaging/messaging-telegram.gtpack"
    - "providers/messaging/messaging-webex.gtpack"

packs:
  - "packs/pack2.gtpack"
  - "packs/pack1.gtpack"           # in dev, operator can build pack dirs into gtpack

env_passthrough:
  - OTEL_EXPORTER_OTLP_ENDPOINT
  - OTEL_RESOURCE_ATTRIBUTES
  - RUST_LOG

policy:
  source:
    tenant_gmap: "tenants/tenant1/tenant.gmap"
    team_gmap: "tenants/tenant1/teams/team1/team.gmap"
  default: forbidden


This file is what dev up and demo up use to start runtime services.

CLI design
greentic-operator dev

init

scan

sync

tenant add|rm|list

team add|rm|list

allow|forbid

up|down|status|logs

setup (optional PR, can come after demo is stable)

greentic-operator demo

build (creates demo bundle)

up|down|status|logs

doctor (optional: run greentic-pack doctor for packs in bundle)

Repo layout (Rust modules)
greentic-operator/
  Cargo.toml
  README.md
  src/
    main.rs
    cli.rs

    project/
      mod.rs
      layout.rs          # paths, defaults
      scan.rs            # find packs/providers/tenants
      resolve.rs         # generate resolved manifests

    gmap/
      mod.rs
      parse.rs
      eval.rs
      edit.rs            # allow/forbid safe edits + stable ordering

    demo/
      mod.rs
      build.rs           # copy packs/providers/tenants + resolved into bundle
      paths.rs

    services/
      mod.rs
      runner.rs          # start/stop external processes, pidfiles, logs
      nats.rs            # docker start/stop
      messaging.rs       # spawn greentic-messaging serve using resolved manifest
      cloudflared.rs     # optional later (stub now)

    util/
      fs.rs
      process.rs
      serde_yaml.rs

  tests/
    gmap_eval.rs
    resolve_manifest.rs
    demo_build.rs

PR split (what to implement, in order)
OP-PR-01 — New repo + project layout + dev init/scan

Deliverable: greentic-operator dev init creates skeleton; dev scan prints summary.

Scope

New greentic-operator repo with Clap CLI

Project layout constants (providers/, packs/, tenants/, state/)

Scanner that:

lists provider gtpacks by domain

lists packs (dirs and .gtpack)

lists tenants/teams

Minimal README.md with quickstart

Acceptance criteria

greentic-operator dev init creates full tree + default _ = forbidden gmap for tenants/default

greentic-operator dev scan prints discovered packs/providers/tenants

OP-PR-02 — .gmap parsing + evaluation + allow/forbid editor

Deliverable: deterministic policy system you can trust in demos.

Scope

gmap::parse (ignore blank lines/comments)

gmap::eval with specificity precedence

dev allow / dev forbid modifies the right file:

--tenant <t> required

optional --team <team>

--path <pack[/flow[/node]]> and policy

Stable ordering on write:

keep comments where possible

else rewrite in canonical order: _, pack, pack/_ , pack/flow, pack/flow/node

Tests

tests/gmap_eval.rs: precedence + overlay tenant/team

tests/gmap_edit.rs: add rule, idempotent update

Acceptance criteria

You can reproduce “most-specific wins”

Edits are deterministic (no noisy diffs)

OP-PR-03 — Resolve manifests (dev sync)

Deliverable: dev sync writes state/resolved/*.yaml for each tenant/team.

Scope

project::resolve:

enumerate tenants/teams

generate resolved manifests

Decide pack resolution rules:

In dev: include pack dirs as-is for scan, but for runtime prefer .gtpack

simplest MVP: if pack is a dir, just include its path and let later stages build it

(or, if greentic-pack build exists and is stable, build dirs to .gtpack into state/)

MVP recommendation: don’t build yet in OP-PR-03. Just resolve paths and get the loop working.

Tests

tests/resolve_manifest.rs: ensures correct manifest emitted per tenant/team, with relative paths preserved if project_root is relative.

Acceptance criteria

dev sync generates manifests for:

tenants with no teams (tenant only)

tenants with teams (tenant+team variants)

Manifest is stable

OP-PR-04 — dev up/down/status/logs (local stack)

Deliverable: one-command demo stack startup from project directory.

Scope

Process supervisor with pidfiles:

state/pids/*.pid

logs under state/logs/*.log

Start (minimum):

NATS via docker (if you already use it)

greentic-messaging serve ... processes

How operator tells messaging what to load:

easiest: operator passes --tenant, optional --team, and --pack <...> for each pack from resolved manifest

also pass --packs-root if used, but prefer explicit --pack list for determinism

MVP command:

greentic-operator dev up --tenant tenant1 [--team team1]

uses state/resolved/tenant1.team1.yaml

starts messaging ingress/egress/subscriptions (or just pack kind if that’s your current mode)

Acceptance criteria

dev up starts services and records PIDs

dev down stops them reliably

dev status shows running/not running

dev logs tails the logs

OP-PR-05 — demo build + demo up

Deliverable: produce demo-bundle/ that runs anywhere.

Scope

demo build --out demo-bundle [--tenant ... --team ...]

copies:

all referenced provider packs

all referenced packs

tenants gmaps

writes demo-bundle/resolved/*.yaml with relative paths

writes greentic.demo.yaml (bundle metadata)

demo up --bundle demo-bundle --tenant ... [--team ...]

behaves like dev up but reads from bundle

Tests

tests/demo_build.rs: verify output structure and resolved manifest paths are relative

Acceptance criteria

You can zip demo-bundle/ and run on another machine (assuming runtimes exist)

Demo does not read from original project dir

---

Decisions (answers to overall questions)

1) Default tenant name for dev init + initial .gmap

Use `default` as the dedicated tenant name.
Reason: clean “works out of the box” path and matches “default demo tenant”.

dev init should create:

tenants/default/tenant.gmap containing:

_ = forbidden

Optional (nice): create tenants/default/teams/default/team.gmap too, but not required for MVP.

2) dev scan output format: structured or human summary

Default to human summary. Add `--format text|json|yaml` (or `--json`) for structured output.
Reason: humans want readable output; automation wants structured output.

3) Preferred crate/binary name to invoke greentic-messaging + flags

Treat runtime binary name as configurable in greentic.yaml, default `greentic-messaging`.
Use current flags:

greentic-messaging serve --tenant <TENANT> [--team <TEAM>] --no-default-packs --pack <PATH>... pack

Always pass explicit `--pack` list from resolved manifest.

4) NATS in dev up

Make NATS optional, default ON.
`dev up` starts NATS unless `--no-nats`.
If docker is unavailable, warn clearly; fail only if required by chosen runtime mode.
Allow future override `--nats-url <URL>`.

5) OP-PR-03 pack resolution rules

Keep MVP “no build”. Do not call greentic-pack build in OP-PR-03.
Resolve pack dirs and .gtpack paths; include paths in manifest.

6) Demo build copy rules

Providers: copy all under providers/** by default; optional `--only-used-providers` later if needed.
Packs: copy only packs referenced by selected tenant/team resolved manifest (or all if no filters).
Resolved manifests in bundle must use relative paths.

Demo reliability guard:
demo build should refuse pack directories unless `--allow-pack-dirs` is passed (then copy dirs but warn not portable).
This prevents non-portable demo bundles.

Acceptance criteria / tests updates

- dev scan `--format json` covers stable output keys.
- dev up uses `--no-default-packs` and explicit `--pack` list.
- demo build fails clearly on pack directories unless `--allow-pack-dirs` (or equivalent) is set.

README.md skeleton (drop-in)

Include this verbatim in README.md:

# greentic-operator

Greentic Operator orchestrates a project directory for demos and local development.
It manages tenants/teams, access mapping (.gmap), pack/provider discovery, resolved manifests, and starting local runtime services.

## Quickstart (dev)

```bash
mkdir my-demo && cd my-demo
greentic-operator dev init
greentic-operator dev tenant add tenant1
greentic-operator dev team add --tenant tenant1 team1
# drop provider packs into providers/messaging/
# drop packs into packs/
greentic-operator dev sync
greentic-operator dev up --tenant tenant1 --team team1
greentic-operator dev logs

Access mapping (.gmap)

Rules are line-oriented:

<path> = <policy>

Paths:

_ default

pack_id, pack_id/_, pack_id/flow_id, pack_id/flow_id/node_id

Policies (MVP):

public

forbidden

Team rules override tenant rules.

Demo bundles
greentic-operator demo build --out demo-bundle --tenant tenant1 --team team1
greentic-operator demo up --bundle demo-bundle --tenant tenant1 --team team1


---

# What to tell Codex (one prompt to generate all PRs)
This matches your “do as much as possible without asking repeatedly” preference:

```md
Create a new repo `greentic-operator` implementing dev + demo orchestration for Greentic.

Implement PRs OP-PR-01..OP-PR-05 as described:

OP-PR-01: Clap CLI with `dev init` and `dev scan`; create project skeleton layout.
OP-PR-02: `.gmap` parser/evaluator and `dev allow`/`dev forbid` that edits gmap deterministically; tests for precedence + overlay.
OP-PR-03: `dev sync` generating `state/resolved/<tenant>[.<team>].yaml` resolved manifests.
OP-PR-04: `dev up/down/status/logs` starting/stopping services with pidfiles+logs; at minimum start NATS via docker and start `greentic-messaging serve` using the resolved manifest packs and tenant/team.
OP-PR-05: `demo build` producing a portable `demo-bundle/` with providers/packs/tenants/resolved; `demo up` runs from bundle.

Keep production hooks minimal: design so `resolved manifests` remain stable and are the only runtime input, but do not implement production deployment.

Do as much as possible without asking permission; only ask if a destructive change is required.
Add a clean README with quickstart instructions.
Include unit/integration tests for gmap and demo build outputs.
