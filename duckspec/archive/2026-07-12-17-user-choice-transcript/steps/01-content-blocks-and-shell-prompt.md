# Content blocks and shell prompt

Add host user-choice content blocks to the session model and keep question text on the
live user-choice shell source.

## Tasks

- [x] 1. Add `ContentBlock::UserChoiceQuestion { text }` and
         `ContentBlock::UserChoiceAnswer { text }` in
         `crates/duckboard/src/chat_store.rs`; fix exhaustive matches that compile-fail

- [x] 2. Extend `FastResponseSource::UserChoice` with `prompt: Option<String>` in
         `crates/duckboard/src/fast_response.rs`; update `from_user_choice` to take and
         store prompt

- [x] 3. In `apply_user_choice_request` (`crates/duckboard/src/area/interaction.rs`), stop
         discarding `prompt` and pass it into `from_user_choice`

- [x] 4. Update call sites and tests that construct `UserChoice` / `from_user_choice` for
         the new prompt field

- [x] 5. @spec chat/persistence User choice content: User-choice question and answer blocks round-trip through persist and load

- [x] 6. @spec chat/persistence User choice content: A legacy session without user-choice content still loads
