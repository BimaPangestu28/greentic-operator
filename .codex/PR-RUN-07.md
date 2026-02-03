PR-RUN-07 — Make it future-proof for “events providers later”

Goal: Ensure today’s design won’t block the future “events providers” work.

Structure UserEvent so it could later be emitted as a real event:

UserEvent::CardSubmit { action_id, fields }

Keep the demo runner boundary clean:

“interactive” is just one front-end; later you can swap it with messaging events.

Files

Mostly refactors / type naming

Minor docs note: “demo run simulates user events locally”

Notes on scope boundaries (so it doesn’t explode)

Demo run should not pull in provider send/receive at all.

It should execute the same flow engine the operator uses elsewhere—just with an interactive front-end.