# Change list section counts

Show `(N)` counts on Change and Archived section headers in the Changes area, matching
Ideas.

## Context

From `reviews/01-followup-change-list-section-counts.md`. Presentation-only; no new caps.

## Tasks

- [x] 1. In `area/change.rs` `view_list`, compute Change count as live listable
         explorations (`is_on_live_list`) plus active changes; Archived count as
         `archived_entries(...).len()`

- [x] 2. Pass titles `Change  (N)` and `Archived  (N)` into the collapsibles (same spacing
         as Ideas `{label}  ({count})`); leave Overview, Capabilities, Reviews, Steps,
         Files, Changed files unnumbered

- [x] 3. Smoke-check counts match the rows under each section (including Archived with
         only explorations)
