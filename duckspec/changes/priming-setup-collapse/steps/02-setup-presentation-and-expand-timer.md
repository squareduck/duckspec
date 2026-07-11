# Setup presentation and expand timer

Render priming as a collapsible Setup header + user-card body; schedule a generation-gated
re-collapse ~15s after expand.

## Prerequisites

- [ ] @step priming-segment-flag-and-collapse-policy

## Tasks

- [x] 1. Add `view_priming_user_block` and route priming User blocks through it with Setup
         collapsed label helper

- [x] 2. On ToggleCollapse of a priming block, bump `priming_expand_gen` and set
         `pending_priming_recollapse` when expanding; clear when collapsing

- [x] 3. Drain pending re-collapse into delayed `Message::RecollapsePriming` in main;
         apply only when expand generation still matches

- [x] 4. @spec chat/transcript Collapse defaults: Timed re-collapse forces priming collapsed

- [x] 5. @spec chat/transcript Segment presentation: Priming collapsed label uses Setup and line count
