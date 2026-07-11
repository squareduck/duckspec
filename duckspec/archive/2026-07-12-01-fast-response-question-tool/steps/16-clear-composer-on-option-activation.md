# Clear composer on option activation

When a chip option is activated while awaiting a user choice, clear any typed composer
text so a partial custom answer is not left behind for a later send.

## Context

Review `reviews/06-review-implementation-complete.md` finding 2. Chips stay visible while
typing a custom answer; ⌘n answers the chip but left freeform text in the input.

## Tasks

- [x] 1. In `activate_fast_response` in `crates/duckboard/src/area/interaction.rs` (or
         immediately after a successful `UserChoice` option pick): clear `chat_input` and
         rehighlight so typed freeform is discarded

- [x] 2. Unit test: option activation while awaiting with non-empty composer leaves the
         input empty and does not invent a user transcript message
