# Archived browse list and section defaults

Newest-first archived changes, interleaved archived explorations on Change and Dashboard,
and collapsed Archive sections by default.

## Prerequisites

- [x] @step exploration-soft-archive-model
- [x] @step archive-action-and-live-lists

## Tasks

- [x] 1. Reverse `archived_changes` after load from `archive/` in `data.rs` (leave active
         changes ascending)

- [x] 2. Add shared `archived_entries` helper: non–idea-owned archived explorations +
         archived changes, sort by archive date descending

- [x] 3. Render interleaved Archived section on Change list (hover remove on exploration
         rows; section present when either kind non-empty)

- [x] 4. Render same interleave on Dashboard Archived (navigation only, no hover remove)

- [x] 5. Default Ideas Archive section collapsed; keep Change Archived collapsed by
         default

- [x] 6. @spec archive/browse Archived change order: Archived changes list most recent first

- [x] 7. @spec archive/browse Interleaved archived rows: Archived non–idea-owned explorations appear with archived changes

- [x] 8. @spec archive/browse Interleaved archived rows: Mixed archive rows order by archive date descending

- [x] 9. @spec archive/browse Interleaved archived rows: Idea-owned archived explorations stay off Change and Dashboard archived lists

- [x] 10. @spec archive/browse Archived section visibility: Archived section is empty only when both kinds are empty

- [x] 11. @spec archive/browse Archived section visibility: Ideas Archive section starts collapsed

- [x] 12. @spec archive/browse Archived section visibility: Change Archived section starts collapsed
