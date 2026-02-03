PR-OP-02 — Remove any operator-side cardkit knowledge (if present)

Repo: greentic-operator
Goal: The operator should not know about cardkit-specific operations if it can avoid it.

Two options, pick whichever matches your current architecture:

Option A (best): Operator never calls cardkit directly

Operator calls renderer → provider encode → provider send

Cardkit stays internal to providers or is bypassed

Option B (compat): Operator still calls cardkit op, but cardkit is a shim

Keep until you’re ready to simplify

Acceptance criteria

No downgrade/downsample behavior.

Webex/webchat work with v1.3+ cards.