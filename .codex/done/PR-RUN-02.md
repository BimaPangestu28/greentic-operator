PR-RUN-02 — In-memory session/state runner integration

Goal: Actually execute a flow using greentic-runner with in-memory session + state, no messaging providers.

Introduce a DemoRunner that:

Builds runner context from:

tenant/team

resolved pack bundle

initial payload (--input)

Creates an in-memory Session + State store (or whatever your runner expects)

Starts the flow and drives it until:

it produces “await user input” output (adaptive card or “prompt”)

or finishes

Define a minimal “demo event loop” API:

run_until_blocked() -> DemoBlockedOn

submit_user_event(UserEvent) -> ()

Files

src/demo/runner.rs: wraps runner initialization + loop

src/demo/types.rs: DemoBlockedOn, UserEvent, etc.

Optional: src/demo/state_mem.rs if you need a small in-memory impl

Tests

Integration-ish test using fixture pack:

a flow that returns a “blocked” step (even if it’s a simple “prompt” node)

assert runner reaches blocked state and doesn’t crash