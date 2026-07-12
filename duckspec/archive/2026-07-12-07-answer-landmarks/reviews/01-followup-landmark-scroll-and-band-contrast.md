# Followup: landmark scroll and band contrast

User-led pass after implementation: ⌘↓ incomplete on restored sessions, all landmark
arrows dead while streaming, and last-Answer band too subtle.

## Scope

Post-apply followup on `answer-landmarks`: `apply_chat_landmark`, scroll-preservation
wrapper, last-Answer theme, live session restore + streaming.

## Summary

```
| # | sev | lens | title | → next |
| --- | --- | --- | --- | --- |
| 1 | major | soundness | ⌘↓ undone by scroll preservation | /ds-step |
| 2 | critical | soundness | Landmarks dead while streaming | /ds-step |
| 3 | minor | quality | Last-Answer band too subtle | /ds-step |
```

## Issues

### 1. ⌘↓ undone by scroll preservation - soundness/major

**Where:** `crates/duckboard/src/main.rs` — `apply_chat_landmark` HistoryBottom vs
`update_with_scroll_preservation`

**Why:** History top / prev / next set `chat_scroll_overridden`; History bottom only
`snap_to_end` + stick. Snapshot replay can cancel the snap, so restored long sessions need
multiple ⌘↓ presses.

**Action:** Set `chat_scroll_overridden` on HistoryBottom (and re-check all landmark paths
against the wrapper); ensure one ⌘↓ reaches true end.

### 2. Landmarks dead while streaming - soundness/critical

**Where:** Landmark dispatch + stream materialize / stick-to-bottom loop

**Why:** While a turn streams, ⌘-arrows stop doing anything. Likely interaction between
layout churn, stick/snap, scroll override, and/or key capture — worse than multi-press;
full no-op.

**Action:** Reproduce mid-stream; fix so all four landmarks work during streaming
(override + stick rules, and key routing if Captured).

### 3. Last-Answer band too subtle - quality/minor

**Where:** `crates/duckboard/src/theme.rs` — `bg_chat_last_answer`

**Why:** Half-mix toward `bg_surface` is easy to miss; user wants a more prominent
full-width band without card chrome.

**Action:** Strengthen tint (e.g. full `bg_surface` or step toward `bg_elevated` / quiet
accent) in light and dark.

## Outcome

Three agreed fixes before archive: reliable history bottom, landmarks while streaming,
stronger last-Answer band. Specs already cover intent; implementation rework via
`/ds-step` (no new behavior contracts expected unless stream-specific stick rules need a
scenario).
