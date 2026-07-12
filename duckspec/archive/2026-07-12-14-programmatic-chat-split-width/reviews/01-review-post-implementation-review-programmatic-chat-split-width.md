# Post-implementation review: Programmatic chat split width

Reviewed the full chain through helpers and production wiring. Intent is met; residual
`or_default` on one non-show path is the only durable nit.

## Scope

Proposal, design, `layout/content-chat-split` deltas, both steps, and production diffs in
`interaction.rs`, `ideas.rs`, `change.rs`, `main.rs`. Post-implementation; `ds audit`
clean; unit tests for both new scenarios green.

## Summary

```
| # | sev | lens | title | → next |
| --- | --- | --- | --- | --- |
| 1 | minor | fidelity | NewSession still inserts via `or_default` | ignore |
```

## Findings

### 1. NewSession still inserts via `or_default` - fidelity/minor

**Where:** `crates/duckboard/src/area/change.rs:565`

**Why:** Design’s construction table skipped this path; proposal’s “newly created panel
uses live window” is slightly broader. In practice SelectChange/AddExploration already
mint with `for_window`, so NewSession rarely inserts — but a bare insert still seeds
default-window half until door/resize/show.

**Action:** Accept as design-scoped residual, or one-line `or_insert_with(for_window)`
later if you want zero production `or_default` for panels.

## Verdict

Ready to freeze. Force-show and listed construction sites match design; scenarios are
backlinked; no soundness gap on Explore or equal-split rules.
