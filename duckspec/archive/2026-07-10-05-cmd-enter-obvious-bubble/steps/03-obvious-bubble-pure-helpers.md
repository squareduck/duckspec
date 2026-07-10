# Obvious bubble pure helpers

Pure helpers for lifecycle send text and bubble visibility, with unit tests for the non-UI
contracts.

## Prerequisites

- [x] @step llm-only-composer-defaults

## Tasks

- [x] 1. Add pure helpers (new `crates/duckboard/src/obvious_bubble.rs` or colocated with
         `default_prompts`) for empty-send formatting (`bubble_send_text`) and
         `bubble_visible(is_streaming, input_empty, obvious_command)`; wire the module
         from the crate root

- [x] 2. Keep empty-send formatting shared with any remaining heuristic-as-send-form
         helper so the composer list is never seeded from it

- [x] 3. @spec chat/obvious-bubble Lifecycle send text: Bare skill name formats with leading slash

- [x] 4. @spec chat/obvious-bubble Lifecycle send text: Already-slashed command is preserved

- [x] 5. @spec chat/obvious-bubble Lifecycle send text: Absent command yields no send text

- [x] 6. @spec chat/obvious-bubble Bubble visibility: Idle empty composer with command shows bubble

- [x] 7. @spec chat/obvious-bubble Bubble visibility: Streaming hides bubble

- [x] 8. @spec chat/obvious-bubble Bubble visibility: Non-empty composer hides bubble

- [x] 9. @spec chat/obvious-bubble Bubble visibility: No command hides bubble

- [x] 10. @spec chat/obvious-bubble Bubble visibility: Oneshot pending does not hide bubble when otherwise visible
