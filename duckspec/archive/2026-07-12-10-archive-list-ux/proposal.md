# Archive list UX

Make archived work easier to browse in duckboard: newest first, explorations archiveable
and interleaved with changes, and archive sections closed by default.

## Motivation

Archived changes load oldest-first, so recent work sinks to the bottom of a long list.
Explorations can only be hard-removed; there is no way to put a finished brainstorm out of
the way and still find it later. Ideas already sort newest-first, but their Archive
section opens expanded, so the list column starts noisy.

Why now: archive volume is already high in real projects, and exploration archive is a
duckboard-only gap next to the existing change and idea archive paths.

## Intent

- Archived changes appear most recent first (archive date from folder naming)

- Live explorations can be archived from the same single hover control used today; once
  archived, that control removes them

- Archived explorations appear in the Changes (and matching) Archived list,
  date-interleaved with archived changes

- Ideas Archive stays newest-first and starts collapsed

- Archived sections on the Changes and Ideas screens start closed

## Non-goals

- CLI / filesystem archive for explorations (`ds archive`, `duckspec/archive/`)
- Mixing ideas into the Changes Archived list
- Redesigning Remove/Archive chrome beyond the single hover action rule
- Sorting or collapsing unrelated list sections (active changes, inbox, etc.)
