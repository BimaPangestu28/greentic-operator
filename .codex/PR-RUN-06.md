PR-RUN-06 — Polish, discovery helpers, docs

Goal: Make it discoverable and “feels done”.

Add optional discovery commands (very small, very helpful):

demo list-packs

demo list-flows --pack <name>

Improve console output:

Show flow end result + exit status

On errors: list valid field/action ids when relevant

Docs:

docs/demo-run.md (or extend existing docs)

Examples:

run default flow

run specific flow with input file

fill a form and click

Tests

“smoke” test for list-packs/list-flows with fixtures