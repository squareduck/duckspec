# Table band find and link paint

Paint find/search match highlights and cmd-hover link underlines on table visual bands so
chat find and link affordances work on GFM table cells.

## Prerequisites

- [x] @step hit-test-chrome-and-chat-enable

## Context

Addresses findings in `reviews/01-post-implementation-review.md` (find/search highlights
never paint on table bands; link hover underline skipped on table bands).

The table paint path in `crates/duckboard/src/widget/text_edit/render.rs` early-
`continue`s after chrome / selection / cell text / rules, so the prose-only match-range
and link-underline passes never run for those visual rows. Chat enables both
`.md_tables(true)` and `.highlights(...)` on the same `TextEdit`; for read-only blocks
highlights are the only find affordance.

Reuse the byte→fragment intersection already used by `paint_table_selection`
(~`paint_table_selection` in `render.rs`).

## Tasks

- [x] 1. On the table band draw path, paint fragment-clipped quads for `highlight_ranges`
         that intersect the current cell fragment (muted search match background)

- [x] 2. Paint a stronger accent fill for `current_highlight` on the same fragment-clipped
         basis (mirror prose find styling)

- [x] 3. When `link_hover` overlaps a cell fragment’s source char range on this visual
         row, draw the accent underline under the overlapping slice (same segments style
         as the prose path)

- [x] 4. Keep paint order sensible: row bg → match highlights → selection → cell text →
         rules → link underline (or equivalent so selection and text remain readable)

- [x] 5. Smoke-check: find a string inside a chat table cell (highlight visible);
         cmd-hover a URL/path in a cell (underline + open still works)
