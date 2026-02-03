PR-OP-01 — greentic-operator consumes renderer as a library (but stays orchestration-only)

Repo: greentic-operator
Goal: Operator doesn’t implement rendering; it just calls the renderer crate from providers.

Deliverables

Add dependency on greentic-messaging-renderer (sourced from the providers workspace via path now but set to version 0.4)

Operator send pipeline:

builds a message/envelope

calls render_plan_*

then calls provider encode/send steps as before

Important boundary note

This still keeps operator “orchestration only” because:

All rendering logic + tests live in greentic-messaging-providers.

Operator just invokes a stable library function.

Acceptance criteria

Operator produces the same outputs but now uses shared renderer.

Operator does not carry renderer tests.