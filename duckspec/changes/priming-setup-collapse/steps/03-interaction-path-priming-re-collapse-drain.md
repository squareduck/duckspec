# Interaction-path priming re-collapse drain

Wire `take_pending_priming_recollapse` into the live chat `Message::Interaction` path so
Setup expand schedules the 15s timer.

## Prerequisites

- [ ] @step setup-presentation-and-expand-timer

## Context

Chat column wraps as `Message::Interaction` (`view_area_three_column` →
`interaction::view_column`). That arm early-returns `route_interaction` and never hits the
fall-through `take_pending_priming_recollapse` at the end of `update`. ToggleCollapse sets
`pending_priming_recollapse` but the sleep task is never scheduled. Change/Caps/Codex
message paths that fall through already drain; the UI click path does not.

See review `01-review-post-implementation-review-priming-setup-collapse`.

## Tasks

- [x] 1. After `route_interaction` handles a message (and any other Interaction
         early-return that can set `pending_priming_recollapse`), batch
         `take_pending_priming_recollapse` so the pending flag is drained into a delayed
         `RecollapsePriming` task on the same tick

- [x] 2. Keep fall-through drains so double-drain is a no-op (flag already taken); do not
         regress `take_pending_chat_snap` behavior

- [x] 3. Freeform regression: expanding a priming Setup block sets then consumes
         `pending_priming_recollapse` into a non-empty scheduled task (test drain helper
         and/or ToggleCollapse + drain on a session fixture)
