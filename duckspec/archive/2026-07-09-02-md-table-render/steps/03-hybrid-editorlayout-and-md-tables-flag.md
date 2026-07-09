# Hybrid EditorLayout and md_tables flag

Wire the pure table layout into `TextEdit` behind an opt-in `md_tables` flag: hybrid
visual rows (prose wrap + table bands), separator lines take no visual rows, and content
width / `scroll_x` when a table overflows the pane.

## Prerequisites

- [x] @step table-layout-kernel-fit-wrap-map

## Tasks

- [x] 1. Add `md_tables: bool` and `TextEdit::md_tables` builder (default `false`)

- [x] 2. Introduce `EditorLayout` (or equivalent) that merges `WrapLayout` for non-table
         lines with `TableLayout` bands for recognized regions

- [x] 3. Map separator source lines to zero visual rows; header/body rows use table row
         heights

- [x] 4. Set content width to `max(pane_chars, max table total_width_chars)` and allow
         `scroll_x` when wider than the pane (even with word wrap)

- [x] 5. Use `EditorLayout` for height measurement and draw row iteration when `md_tables`
         is on (hit-test still temporary if needed; step 04 completes mapping)
