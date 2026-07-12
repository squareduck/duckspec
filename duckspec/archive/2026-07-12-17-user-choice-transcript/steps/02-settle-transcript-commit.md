# Settle transcript commit

On pick or freeform settle, append host question and answer messages; on cancel, commit
nothing. Keep agent wire in-band.

## Prerequisites

- [x] @step content-blocks-and-shell-prompt

## Tasks

- [x] 1. Add `settle_user_choice_transcript` in
         `crates/duckboard/src/area/interaction.rs`: resolve prompt from shell; append
         Assistant `UserChoiceQuestion` when non-empty; append User `UserChoiceAnswer`
         with answer text (no hotkey); clear shell

- [x] 2. Wire option pick in `activate_fast_response` (UserChoice arm): in-band
         `answer_user_choice`, resolve option label (fallback to id), call settle helper

- [x] 3. Wire freeform-while-awaiting submit: in-band custom answer, settle with freeform
         text, clear composer as today

- [x] 4. Ensure cancel / `clear_user_choice_shell` alone does not commit question or
         answer blocks

- [x] 5. @spec chat/fast-response Ephemeral chips: Visible chips are not a stored user message

- [x] 6. @spec chat/fast-response Question activation: Option activation answers in-band and commits host question and answer

- [x] 7. @spec chat/fast-response Freeform while awaiting: Freeform submit completes the pending choice as a custom answer

- [x] 8. @spec chat/fast-response Settled choice transcript: Settle with a prompt commits question then answer without a hotkey

- [x] 9. @spec chat/fast-response Settled choice transcript: Settle without a prompt commits answer only

- [x] 10. @spec chat/fast-response Settled choice transcript: Cancel commits no choice blocks
