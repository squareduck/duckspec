# Hit-test chrome and chat enable

Finish editor integration: table-aware hit-test and visual caret motion, fragment-clipped
selection paint, header/zebra/rules chrome, and enable `.md_tables(true)` on chat and tool
message bodies.

## Prerequisites

- [x] @step hybrid-editorlayout-and-md-tables-flag

## Context

From step 03 (hybrid layout wiring):

- `TextEdit::md_tables(bool)` exists (default `false`). Chat call sites still need
  `.md_tables(true)`.
- `EditorLayout` in `text_edit/render.rs` merges prose wrap + `md_table::TableLayout`;
  separators contribute 0 visual rows; draw already paints cell fragments (no `|` glyphs)
  for table bands and skips prose selection/syntect on those bands.
- `pixel_to_pos_wrapped` already routes table bands through `md_table::visual_to_source`
  when hybrid is active. Visual up/down still use wrap-only targets when hybrid is on
  (arrow keys across table bands need `EditorLayout`-aware motion).
- Selection highlight, header/zebra fills, and column/row rules are not painted yet.

## Tasks


- [x] 1. Route `pixel_to_pos` (and visual up/down) through the hybrid layout’s table map
         for positions in table bands

- [x] 2. Paint selection highlights as fragment-clipped quads inside table cells

- [x] 3. Draw table chrome from layout roles: header/zebra row backgrounds, column/row
         rules, cell text only (no pipe glyphs)

- [x] 4. Enable `.md_tables(true)` on chat user/assistant/system and tool body `TextEdit`s
         in `agent_chat`

- [x] 5. Smoke-check a chat message with a GFM table: columns align, wrap fits the pane,
         selection/copy still yield source markdown
