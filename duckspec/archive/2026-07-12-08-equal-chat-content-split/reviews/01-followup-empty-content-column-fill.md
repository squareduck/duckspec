# Followup: empty content column fill

User-led pass after implement: no open tabs should hide the content column in every
three-column area so chat fills; opening a list item restores content.

## Scope

Post-implementation followup on `equal-chat-content-split`: proposal no-content bound,
design `show_content`, `caps/layout/content-chat-split` Content-hidden fill, and live
Change with no tabs.

## Summary

```
| # | sev | lens | title | → next |
| --- | --- | --- | --- | --- |
| 1 | major | fidelity | Empty content column does not give chat fill | /ds-spec |
```

## Issues

### 1. Empty content column does not give chat fill - fidelity/major

**Where:** `view_area_three_column` `show_content` (`crates/duckboard/src/main.rs`);
`layout/content-chat-split` Content-hidden fill; proposal no-content non-goal

**Why:** Real changes (and other three-column areas) with no open tabs still reserve half
the free space for an empty “Select an item…” shell. Exploration-only hide was an
intentional narrow, but live use shows wasted space and under-delivers “no content → full
chat.”

**Action:** Spec and implement: hide content whenever there are no tabs (preview + file
tabs) in any three-column area; interaction fills. Selecting a list item that opens a tab
restores the content column (equal or customized width as today).

## Outcome

Agreed to broaden no-content from exploration-no-tabs only to **no open tabs in any
three-column area**, with restore when a list selection opens a tab. Ready for `/ds-spec`
then steps/apply. Not archive-ready until this lands.
