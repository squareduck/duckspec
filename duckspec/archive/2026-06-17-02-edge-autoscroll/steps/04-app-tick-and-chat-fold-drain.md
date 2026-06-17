# App tick and chat-fold drain

Drive the terminal step from a gated 60fps subscription, and drain a chat message's
past-the-fold `AutoScroll` request into an absolute scroll on the outer chat scrollable.

## Prerequisites

- [ ] @step text-editor-drive
- [ ] @step terminal-drive

## Tasks

- [x] 1. In `area/interaction.rs`, add `pub pending_chat_autoscroll:
         Option<f32>` to the
         chat session state (with doc comment) and initialize it to `None` in its
         constructor.

- [x] 2. In the `agent_chat::Msg::ChatAction` handler, intercept
         `EditorAction::AutoScroll { dy }`: accumulate it into `pending_chat_autoscroll`,
         drop `stick_to_bottom` so a later streaming snap can't fight the scroll, and
         return early.

- [x] 3. In `main.rs`, add the `TerminalAutoscrollTick` message variant and an `update`
         arm that steps every terminal whose `is_drag_autoscrolling()` is true across
         `state.interactions`.

- [x] 4. In `subscription`, push an
         `iced::time::every(Duration::from_millis(16))
         .map(|_| Message::TerminalAutoscrollTick)`
         gated on a new `any_terminal_autoscrolling(state)` helper so it only fires while
         a terminal drag holds the pointer past an edge.

- [x] 5. Add the `has_pending_chat_autoscroll(state)` and
         `take_pending_chat_autoscroll(state) -> Task<Message>` helpers: the latter drains
         each session's delta into `y = (last_chat_offset_y + dy).max(0.0)`, updates
         `last_chat_offset_y`, and issues
         `scroll_to(CHAT_SCROLLABLE_ID,
         AbsoluteOffset { x: 0.0, y })`.

- [x] 6. At the chat-scroll reconciliation point in `update`, when
         `has_pending_chat_autoscroll` is true, batch the drain task and skip the snapshot
         replay (replaying the pre-update offset would undo the deliberate scroll).
