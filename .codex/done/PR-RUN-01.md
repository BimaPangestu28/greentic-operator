PR-RUN-01 — CLI skeleton and pack selection

Goal: Add a new command surface that can select a pack in ./packs, tenant, optional team, optional flow, optional input.

Add greentic-operator demo run subcommand:

--packs-dir default ./packs

--pack <name> required

--tenant <id> required

--team <id> optional (use operator default if omitted)

--flow <flow_id> optional (default flow from pack manifest if omitted)

--input <json|yaml|@file> optional

Implement:

Resolve pack path: <packs-dir>/<pack>/

Load pack manifest; resolve default flow

Validate flow exists in pack bundle

Parse input (inline JSON/YAML or @path)

Output a “run header” with resolved values:

pack name/path, tenant/team, flow id, input source (inline/file/none)

Files (example)

src/cli.rs (or wherever subcommands live): new demo run args + wiring

src/demo/mod.rs + src/demo/args.rs (optional): isolate demo code

src/demo/pack_resolve.rs: pack + manifest + default-flow resolution

src/demo/input.rs: parse input / @file

Tests

Unit tests:

pack resolution and “default flow missing” error

input parsing: JSON inline, YAML inline, @file

Add a tiny fixture pack under tests/fixtures/packs/demo_pack_min/ with a manifest + one flow.