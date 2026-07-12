# Quiet activity chrome

Implement flat secondary headers with kind icons and remove Activity tool-card surfaces so
only User and Answer stay prominent.

## Prerequisites

- [x] @step transcript-presentation-deltas

## Tasks

- [x] 1. Add `crates/duckboard/assets/icon_thinking.svg` (thought bubble + two dots) and
         `icon_tool.svg` (open-end wrench) in Lucide stroke style (`currentColor`, 24×24)

- [x] 2. Add `secondary_segment_header` in `crates/duckboard/src/widget/agent_chat.rs`
         (chevron + muted SVG icon + muted label) and wire Thinking + Activity headers
         through it

- [x] 3. Rewrite `view_activity_block` to match Thinking layout: no `chat_tool_card_*`
         styles, no border stack; transparent body with
         `base_color(theme::text_secondary())`; padding aligned with Thinking

- [x] 4. Mute Activity/ToolUse header ink via `block_header_color` → `text_muted`

- [x] 5. Delete unused `chat_tool_card_frame`, `chat_tool_card_header_open`,
         `chat_tool_card_header_alone`, and `chat_tool_card_body` from
         `crates/duckboard/src/theme.rs`

- [x] 6. Run duckboard tests that cover transcript segments/labels; fix any fallout
