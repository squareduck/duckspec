# Apply handoff dual critique

User-led pass after all steps: apply's all-done handoff listed only `/ds-archive` and
`/ds-review`, so `/ds-followup` was missing from suggested next actions.

## Scope

Post-implementation followup on change `review-followup-workflow`: shipped apply template
handoff vs dual-critique chrome (review + followup). Content only —
`crates/duckspec/content/templates/apply.md`.

## Summary

```
| # | sev | lens | title | → next |
|---|-----|------|-------|--------|
| 1 | major | fidelity | Apply all-done handoff omits /ds-followup | amended |
```

## Issues

### 1. Apply all-done handoff omits /ds-followup — fidelity/major

**Where:** `crates/duckspec/content/templates/apply.md` Handoff — "All steps complete and
scoped audit is clean"

**Why:** Chrome and the change design treat review and followup as peer critique modes
after implementation. Apply still ranked only archive + review, so agents finishing the
last step never surface user-led followup as a next action.

**Action:** Amended the all-done suggested list to `/ds-review` and `/ds-followup` (at
most two ranks preserved). Critique-before-archive is the intended path; `/ds-archive`
noted as still available when ready to freeze.

## Outcome

Apply handoff matches dual-critique intent for post-step next actions. No further plan/cap
work required for this issue. Change remains archive-ready once any remaining review is
optional polish.
