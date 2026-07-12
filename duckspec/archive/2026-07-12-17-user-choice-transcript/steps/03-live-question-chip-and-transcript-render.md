# Live question chip and transcript render

Show the live question chip above options and render settled Q→A blocks with the same chip
language.

## Prerequisites

- [x] @step settle-transcript-commit

## Tasks

- [x] 1. Add `theme::chat_fast_response_chip_question` (chip geometry, `bg_chat_area`
         fill, soft border; not accent-tinted) in `crates/duckboard/src/theme.rs`

- [x] 2. In `view_fast_response` (`crates/duckboard/src/widget/agent_chat.rs`), when
         source is UserChoice with non-empty prompt, paint a non-clickable question chip
         above numbered option chips

- [x] 3. Add `TranscriptSeg::UserChoiceQuestion` / `UserChoiceAnswer`; map both content
         blocks in `build_transcript_segments`; render with question and answer chip
         styles (settled answer without hotkey)

- [x] 4. Handle new content blocks in `build_history_preamble` and any other exhaustive
         `ContentBlock` matches still open

- [x] 5. @spec chat/fast-response Live question chip: Non-empty prompt shows a question chip above options

- [x] 6. @spec chat/fast-response Live question chip: Empty prompt omits the question chip
