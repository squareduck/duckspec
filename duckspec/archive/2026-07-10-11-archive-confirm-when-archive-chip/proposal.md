# Archive confirm when archive chip shows

Extend obvious-chrome Confirm/Reject so a non-empty active-change session shows the gate
whenever lifecycle includes `/ds-archive` — covering the archive dry-run “Confirm?” moment
with no disk change and no chat parsing.

## Motivation

When the user activates `/ds-archive`, the agent dry-runs archival and asks whether to
proceed. Disk state does not change until they confirm, so chrome stays on the “all steps
done, no review” arm: lifecycle chips only, no Confirm/Reject. Typing Confirm by hand is
the only path.

Chrome cannot inspect chat content. The simplest filesystem-faithful proxy is the same
condition that already surfaces `/ds-archive`: if that chip would show on a non-empty
session, also show the gate. Pre-step and post-review arms already include archive with
Confirm via other flags; the gap is all-steps-complete with no review.

## Scope

```
caps/chat/
└── obvious-bubble/   (modified — gate when archive is in lifecycle)
```

### New capabilities

None.

### Modified capabilities

- `chat/obvious-bubble` — For an active change with a non-empty session, include affirm
  Confirm and decline Reject when lifecycle contains `/ds-archive` (in addition to today’s
  `has_review || !has_steps`). Empty sessions remain lifecycle-only. Affirm still wins ⌘↩
  when both gate and lifecycle are present (`/ds-archive` via ⌘n or chip click).

### Out of scope

- Chat or transcript parsing to detect “Confirm?” prompts
- Session-local latches after sending `/ds-archive`
- Lifecycle ladder composition or `session/scope` next-stage text
- Input hints / auto-messages settings
- Exploration Create change and archived Commit paths
- Opening the gate for open-steps-without-review (still lifecycle-only)

## Impact

```
build_obvious_chrome
  all steps done, no review, nonempty session
    before: /ds-archive, /ds-review          (no gate)
    after:  /ds-archive, /ds-review + Confirm + Reject
```

- duckboard: extend gate in `area/change.rs` (`build_obvious_chrome`) and composition
  tests; cap deltas for composition rule and doc table row “All steps done, no reviews”

- Arms that already list `/ds-archive` with Confirm (caps-no-steps, post-review) stay
  behaviorally the same; the rule is unified rather than re-specialized
