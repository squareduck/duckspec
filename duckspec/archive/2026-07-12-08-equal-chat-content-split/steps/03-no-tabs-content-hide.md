# No-tabs content hide

Hide the content column whenever there are no open tabs in any three-column area; chat
fills; opening a tab restores content. Cover the two new scenarios.

## Prerequisites

- [x] @step window-resize-and-view-wiring

## Context

Followup `reviews/01-followup-empty-content-column-fill.md`: broaden no-content beyond
exploration-only. `show_content` today is
`(!is_exploration || has_tabs) &&
!content_collapsed` — change to tab presence for all
three-column areas.

## Tasks

- [x] 1. Extract pure `show_content_column(has_tabs, content_collapsed) -> bool` (true
         only when there is at least one open tab and content is not collapsed); use it
         from `view_area_three_column` instead of exploration-only gating

- [x] 2. Wire `show_content` so any three-column area with no preview and no file tabs
         hides content and sizes interaction as Fill; keep handle visibility coherent
         (chat can still toggle when the panel is available)

- [x] 3. Confirm list selection that opens a tab re-shows content via `has_tabs` (and
         existing `content_collapsed` clear on open-content if still needed); no
         special-case exploration branch for hide

- [x] 4. @spec layout/content-chat-split Content-hidden fill: No open tabs hides content column

- [x] 5. @spec layout/content-chat-split Content-hidden fill: Opening a tab restores content column
