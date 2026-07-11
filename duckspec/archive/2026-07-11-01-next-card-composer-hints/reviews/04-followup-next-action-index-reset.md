# Followup: next-action index reset

User-led pass: after Tab-cycling on one trailing `next` card, the next turn’s ghost still
used the old active index (e.g. `reject` instead of `confirm`).

## Scope

Post-apply tryout on `next-card-composer-hints`: `next_actions` / `next_action_idx`
refresh on turn complete.

## Summary

```
| # | sev | lens | title | → next |
| --- | --- | --- | --- | --- |
| 1 | major | quality | Next-action index not reset on list refresh | /ds-step |
```

## Issues

### 1. Next-action index not reset on list refresh - quality/major

**Where:** `crates/duckboard/src/area/interaction.rs` — `refresh_next_actions` (rebuilds
list then only clamps index)

**Why:** Ranked trailing `next` actions are meant to default to the first token. Clamping
preserves a prior Tab index when the list is replaced, so a later card can ghost a
secondary action (e.g. `reject`) after the user cycled on an earlier card.

**Action:** Reset active index to 0 when the next-action list changes (or on every refresh
after a new assistant turn). Keep clamp only if needed for in-list length shrink without a
full replace. `/ds-step` / `/ds-apply`.

## Outcome

Agreed sticky-index fix. Suggested next: `/ds-step`. Not archive-ready until it lands.
