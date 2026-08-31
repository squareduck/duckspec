# Strip answer-thrash cancel

Keep live-draft replace; delete the budget, trip, stop notice, and cancel hook.

## Tasks

- [x] 1. Strip the budget/trip from `apply_answer_content_delta` and
         `apply_reasoning_content_delta`; keep replace-as-is

- [x] 2. Delete `ANSWER_REPLACE_BUDGET`, `ANSWER_THRASH_STOP_NOTICE`,
         `reset_answer_thrash`, and `on_answer_thrash_trip`

- [x] 3. Remove `answer_replace_count` and `answer_thrash_tripped` from `ChatSession`

- [x] 4. Remove the ContentDelta trip-cancel in `main.rs` and every `reset_answer_thrash`
         call site

- [x] 5. Delete the stream-ui thrash tests (exceeding budget; tool use resets)

- [x] 6. @spec chat/stream-ui Answer draft across thought: Reasoning leaves the open answer uncommitted

- [x] 7. @spec chat/stream-ui Answer draft across thought: Answer after reasoning replaces the live draft

- [x] 8. @spec chat/stream-ui Answer draft across thought: Tool use commits the open answer draft

- [x] 9. Delete `thrash_trip_captures_the_kept_draft`

- [x] 10. @spec chat/cancel-resync Draft capture on cancellation: User cancel captures the in-flight draft

- [x] 11. @spec chat/cancel-resync Draft capture on cancellation: Deltas arriving after cancel are part of the captured draft

- [x] 12. @spec chat/cancel-resync Draft capture on cancellation: Cancellation with no in-flight draft records nothing
