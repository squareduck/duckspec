# Previous reply re-align

Add `target_answer_for_reply_jump`, wire `ScrollToAdjacentAnswer` to use it with measured
scroll offset, and cover the three re-align scenarios.

## Tasks

- [x] 1. Add `target_answer_for_reply_jump` in `crates/duckboard/src/widget/agent_chat.rs`
         next to the existing answer-landmark helpers: previous re-aligns to `current`
         when `offset_y > top + VIEWPORT_TOP_EPS`, otherwise `prev_answer_idx`; next is
         always `next_answer_idx`

- [x] 2. In `ScrollToAdjacentAnswer`, capture `translation.y` from the chat scrollable,
         prefer it as `offset_y` in `finish`, and resolve the scroll target via
         `target_answer_for_reply_jump` instead of bare prev/next

- [x] 3. @spec chat/answer-landmarks Previous reply re-align: Viewport below current top targets current Answer

- [x] 4. @spec chat/answer-landmarks Previous reply re-align: At current top previous targets prior Answer

- [x] 5. @spec chat/answer-landmarks Previous reply re-align: Next ignores re-align when below current top

- [x] 6. Run the duckboard unit tests that cover answer-landmark helpers and fix any
         fallout
