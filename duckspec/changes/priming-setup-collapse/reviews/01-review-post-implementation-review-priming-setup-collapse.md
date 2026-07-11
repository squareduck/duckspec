# Post-implementation review: priming Setup collapse

Reviewed proposal → design → caps → steps → code for auto-folding the synthetic first-turn
Setup inject. Collapse defaults and presentation match intent and are tested; the 15s
re-hide path is not reached from the live chat UI.

## Scope

`proposal.md`, `design.md`, `caps/chat/transcript` spec/doc deltas, both steps, and code
under `crates/duckboard` (`agent_chat`, `text_edit/state`, `area/interaction`, `main`).
Post-implementation; `ds audit` clean.

## Summary

```
| # | sev | lens | title | → next |
| --- | --- | --- | --- | --- |
| 1 | critical | soundness | Expand timer never drains on Message::Interaction | /ds-step |
```

## Findings

### 1. Expand timer never drains on Message::Interaction - soundness/critical

**Where:** `crates/duckboard/src/main.rs` — chat column wraps as `Message::Interaction`
(`view_area_three_column` → `interaction::view_column`); `Message::Interaction`
early-returns `route_interaction` without calling `take_pending_priming_recollapse`. Drain
only runs at the fall-through end of `update`.

**Why:** Clicking Setup's chevron sets `pending_priming_recollapse` in
`handle_agent_chat`, but the UI path never schedules the sleep task. Expand works;
auto-hide after ~15s does not. That is a core product requirement (intent + design) left
unshipped. Pure `recollapse_priming` tests pass without covering this wiring gap.

**Action:** Drain `take_pending_priming_recollapse` (and ideally share the same
pending-task pattern as `take_pending_chat_snap`) on every interaction path — e.g. batch
it into `route_interaction`'s return, or stop early-returning `Message::Interaction`
without the pending drains. Add a regression that the pending flag is consumed into a
scheduled task when ToggleCollapse expands priming (unit-test the drain helper if the full
Task is hard to assert).

## Verdict

**Not archive-ready.** Segment flags, first-sight collapse, Setup presentation, and
generation-gated pure re-collapse are sound and backlinked. The timed re-hide is dead on
the primary UI path until the drain is wired through `Message::Interaction`. Fix that,
re-audit, then archive.
