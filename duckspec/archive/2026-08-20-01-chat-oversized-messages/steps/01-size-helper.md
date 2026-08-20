# Size helper

Add `is_oversized`, `display_prefix`, and `maybe_highlight` next to chat rebuild in
`interaction.rs`.

## Tasks

- [x] 1. In `crates/duckboard/src/area/interaction.rs`, add the size helper per design:
         `MAX_LINES` 200, `MAX_CHARS` 16_384, source characters via
         `lines.join("\n").chars().count()`, `is_oversized`, `display_prefix` (exact
         source prefix, no ellipsis line), and
         `maybe_highlight(editor, highlighter, skip)` (`skip || is_oversized` →
         `highlight_spans = None`, else markdown `highlight_lines`)

- [x] 2. @spec chat/oversized-messages Size gate: Over the line cap

- [x] 3. @spec chat/oversized-messages Size gate: Over the character cap on a single long line

- [x] 4. Run focused duckboard tests for the new helper tests and fix failures
