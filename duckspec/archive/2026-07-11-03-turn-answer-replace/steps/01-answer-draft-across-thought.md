# Answer draft across thought

Change apply deltas so reasoning does not commit the answer draft; answer-after-thought
replaces; tools commit; channel switch still materializes.

## Tasks

- [x] 1. Update `apply_reasoning_content_delta` so it does not flush `pending_text` (draft
         stays live; still report kind_switch when an answer draft is open)

- [x] 2. Update `apply_answer_content_delta` so answer-after-thought with a non-empty
         draft clears `pending_text` then appends (replace)

- [x] 3. Keep `ToolUse` / `TurnComplete` `flush_all_pending` behavior so tools and turn
         end still commit the open draft

- [x] 4. @spec chat/stream-ui Answer draft across thought: Reasoning leaves the open answer uncommitted

- [x] 5. @spec chat/stream-ui Answer draft across thought: Answer after reasoning replaces the live draft

- [x] 6. @spec chat/stream-ui Answer draft across thought: Tool use commits the open answer draft

- [x] 7. @spec chat/stream-ui Bounded materialization while streaming: Answer-to-reasoning channel switch materializes without committing the answer
