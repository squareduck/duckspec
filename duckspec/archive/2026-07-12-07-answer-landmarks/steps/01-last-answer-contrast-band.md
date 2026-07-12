# Last Answer contrast band

Add a full-width last-Answer surface and a pure band-target helper, then paint the latest
non-empty Answer in the chat transcript.

## Tasks

- [x] 1. Add `bg_chat_last_answer` and `chat_last_answer_band` in
         `crates/duckboard/src/theme.rs` (full-width, no border/radius)

- [x] 2. Add pure `last_answer_band_target(blocks) -> Option<usize>` (latest non-empty
         `BlockKind::Assistant`) in `crates/duckboard/src/widget/agent_chat.rs`

- [x] 3. Pass the band target into `view_prose_block` and style the matching Answer with
         `chat_last_answer_band` (full width, not `chat_user_card`)

- [x] 4. @spec chat/answer-landmarks Last Answer contrast band: Sole latest non-empty Answer is the band target

- [x] 5. @spec chat/answer-landmarks Last Answer contrast band: Empty latest Answer is not a band target
