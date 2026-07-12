# Post-implementation review: resend-hint-no-first-bind

Reviewed the full chain through code. Product fix is correct and faithful to fork C;
verification of the status-wire predicate is thinner than the pure AND, and requirement
naming still reads like the old “would resend” rule.

## Scope

Proposal, design, `chat/composer-footer` deltas, both steps, and the duckboard touch
points (`agent_chat` helper/tests/view, `interaction` status construction).
Post-implementation; `ds check` / `ds audit` clean.

## Summary

```
| # | sev | lens | title | → next |
| --- | --- | --- | --- | --- |
| 1 | major | quality | Status-wire formula untested; dual scenarios collapse | /ds-step |
| 2 | minor | fidelity | Requirement/scenario titles still say “would be resent” | /ds-spec |
```

## Findings

### 1. Status-wire formula untested; dual scenarios collapse - quality/major

**Where:** `crates/duckboard/src/area/interaction.rs:4279`; tests at
`crates/duckboard/src/widget/agent_chat.rs:2181` and `:2207`

**Why:** The product distinction (stored id ∧ ¬resumable vs unbound) lives only in the
status builder. Both “resume” and “no stored id” `@spec` tests pass the same pure inputs
(`has_messages=true`, `unresumable=false`). Reverting the wire to
`resumable_session_id().is_none()` alone restores the first-bind flash while every unit
test still passes.

**Action:** Extract a pure `has_stored_id && !will_resume` (or equivalent) helper used by
the status builder; give the two hide scenarios distinct inputs via that helper. Optional
but closes the regression hole this change exists to fix.

### 2. Requirement/scenario titles still say “would be resent” - fidelity/minor

**Where:** `duckspec/changes/…/caps/chat/composer-footer/spec.delta.md` — requirement name
and “Hint shown when history would be resent”

**Why:** Prose correctly requires a stored unresumable id; recovery can still re-feed
history with the hint silent. Cold readers of the title still infer the old broader rule.

**Action:** Rename requirement/scenarios to “unresumable stored session” (or similar) in a
light delta; keep GWT bodies.

## Verdict

Ready to ship behaviorally: wire and pure rule match design fork C, preamble/recovery
untouched, suite green. Prefer a small step for wire-level coverage before freeze; naming
polish is optional. Not a rework.
