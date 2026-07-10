# Wrap-safe defaults list UI

Make ready default-prompt rows soft-wrap and grow so long suggestions stay fully readable
and never paint through the next row.

## Prerequisites

- [x] @step oneshot-soft-length-budget

## Tasks

- [x] 1. In `view_default_prompt_list` (`crates/duckboard/src/widget/agent_chat.rs`), drop
         fixed `LINE_HEIGHT` on each suggestion row and marker

- [x] 2. Give prompt text `width(Fill)` and `wrapping(Word)`; set the row to
         `align_y(Start)` and `width(Fill)` so height follows content

- [x] 3. Add light vertical spacing between rows (e.g. `theme::SPACING_XS`); keep padding,
         colors, and `↳` active-only marker as today

- [x] 4. Manually verify a long ready suggestion soft-wraps without overlapping the next
         row and shows full multi-line text (covers the two `manual` presentation
         scenarios)
