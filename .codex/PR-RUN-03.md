PR-RUN-03 — Adaptive card detection and rendering summary

Goal: When the flow outputs an adaptive card, show a human-friendly summary and hold it as the “current card”.

Detect adaptive card outputs in runner results:

Recognize an Adaptive Card payload by schema/version or your envelope type

Add CardView:

Extract inputs (field ids, types, placeholders/labels)

Extract actions (action ids + titles)

Default output when a card appears:

Print “Card received” + summary (same as @show)

Print a hint: available commands

Files

src/demo/card/mod.rs

src/demo/card/parse.rs

src/demo/card/show.rs

Tests

Unit test: parse a sample Adaptive Card JSON (v1.4 etc.) and extract fields/actions