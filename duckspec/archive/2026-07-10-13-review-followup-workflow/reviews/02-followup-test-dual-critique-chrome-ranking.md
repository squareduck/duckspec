# Test dual critique chrome ranking

Test followup on `review-followup-workflow`: invented fidelity gap so the user-led record
path can be exercised end-to-end.

## Scope

Post-implementation test pass on change `review-followup-workflow`. Deepest layer
discussed: lifecycle chrome content for dual critique (`caps/chat/obvious-bubble` and
apply/session handoff templates). No real defect claimed — this document is fixture for
followup workflow testing.

## Summary

```
| # | sev | lens | title | → next |
|---|-----|------|-------|--------|
| 1 | major | fidelity | Apply all-done handoff omits /ds-followup | /ds-step |
```

## Issues

### 1. Apply all-done handoff omits /ds-followup — fidelity/major

**Where:** `crates/duckspec/content/templates/apply.md` Handoff — "All steps complete and
scoped audit is clean"

**Why:** Chrome and the change design treat review and followup as peer critique modes
after implementation. Apply still ranked only archive + review, so agents finishing the
last step never surface user-led followup as a next action.

**Action:** Retarget the all-done suggested list to include `/ds-review` and
`/ds-followup` (at most two ranks preserved). Critique-before-archive is the intended
path; keep `/ds-archive` available when ready to freeze. Plan/code not changed in this
session.

## Outcome

Recorded a synthetic major/fidelity issue matching the prior apply-handoff finding so
`/ds-followup` create + write + handoff can be tested. Plan and product code were not
changed. Suggested next for a real fix would be `/ds-step`; for this test, ignore or keep
the file as fixture.
