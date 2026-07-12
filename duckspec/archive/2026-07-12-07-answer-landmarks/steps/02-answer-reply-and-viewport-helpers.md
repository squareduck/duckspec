# Answer reply and viewport helpers

Pure Answer-anchor list, prev/next resolution, and viewport-current selection used by ⌘←/→
— unit-tested before keybind wiring.

## Prerequisites

- [x] @step last-answer-contrast-band

## Tasks

- [x] 1. Add `answer_block_indices(blocks) -> Vec<usize>` for `BlockKind::Assistant` in
         order

- [x] 2. Add `prev_answer_idx` / `next_answer_idx` (no wrap) over an anchor list and
         optional current block index

- [x] 3. Add `current_answer_for_reply_jumps` from stick-to-bottom, scroll offset, and
         Answer tops (last Answer whose top ≤ offset; else first; stick ⇒ last)

- [x] 4. @spec chat/answer-landmarks Answer reply anchors: Only Answer blocks are reply anchors

- [x] 5. @spec chat/answer-landmarks Answer reply anchors: Prev and next step to adjacent Answer anchors

- [x] 6. @spec chat/answer-landmarks Answer reply anchors: Prev at first and next at last yield no target

- [x] 7. @spec chat/answer-landmarks Viewport current for reply jumps: Stick-to-bottom treats the last Answer as current

- [x] 8. @spec chat/answer-landmarks Viewport current for reply jumps: Scroll offset selects the Answer at or above the viewport top
