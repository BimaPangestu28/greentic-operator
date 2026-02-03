PR-RUN-05 — Implement @back semantics

Goal: Provide @back as a simple, predictable UX feature.

Two reasonable implementations; pick the simplest that fits your runner:

Option A (recommended): Keep a stack of “blocked states”

Store: last blocked card JSON + pending_inputs snapshot + runner “resume token” if you have one

@back restores previous blocked state (does not rewind the actual flow further than the last blocked point)

Option B: If runner supports stepping/replaying deterministically, you can re-run to prior point

More complex; only do if already supported.

Files

src/demo/history.rs: state snapshots

src/demo/repl.rs: handle @back

Tests

REPL state test: set input, then @back restores previous pending values/card