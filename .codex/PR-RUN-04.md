PR-RUN-04 — Interactive REPL with @commands

Goal: Implement the interactive CLI loop and the @ command parser.

REPL loop:

If blocked on card: read user lines, accept only @...

Otherwise: run until blocked/finished

Commands:

@show prints summary + current inputs

@json prints full JSON for current card

@input field=value stores value in pending_inputs[field]

@click action_id submits event and clears pending inputs

@quit exits

Command parser rules:

Trim whitespace

Unknown command prints help and suggests @show

Files

src/demo/repl.rs: loop

src/demo/commands.rs: parser + dispatcher

src/demo/help.rs: help text / usage hints

Tests

Unit tests for command parsing:

@input a=b

@click submit

unknown command

invalid format