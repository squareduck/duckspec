# Cache hybrid EditorLayout

Cache hybrid `EditorLayout` on the text-edit widget’s internal state so layout, update,
and draw share one compute until inputs change.

## Prerequisites

- [x] @step hybrid-editorlayout-and-md-tables-flag
- [x] @step hit-test-chrome-and-chat-enable

## Context

Addresses the layout thrash finding in `reviews/01-post-implementation-review.md`.

Design called for a single `EditorLayout` per frame shared by draw, hit-test, and caret
motion. Today `EditorLayout::compute` runs independently in `layout`, `update`, and `draw`
on `crates/duckboard/src/widget/text_edit/render.rs`, each time re-running `layout_tables`
over all lines.

Cache on `InternalState` keyed by inputs that affect the result: at least `pane_chars`,
`word_wrap`, and a stable identity for `state.lines` (pointer/ version or content
fingerprint already available on the editor if one exists — use the cheapest correct
invalidation). Invalidate when any key changes; keep behavior identical to
always-recompute.

## Tasks

- [x] 1. Add an `EditorLayout` cache field (and key) on `InternalState` for the hybrid
         path

- [x] 2. Provide a get-or-compute helper used by `layout`, `update`, and `draw` when
         `md_tables` is on

- [x] 3. Invalidate / recompute when `pane_chars`, `word_wrap`, or lines identity changes;
         leave the non-hybrid wrap path unchanged unless it naturally shares the helper

- [x] 4. Confirm hybrid unit tests still pass and that separator-zero-height, overflow
         width, and visual up/down still match pre-cache behavior
