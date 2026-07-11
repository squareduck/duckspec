# Custom freeform answer wire

Add host `Custom { text }` choice answer and encode it as an accepted free-text answer for
Grok and allow + free-text answers for Claude (not cancel/skip).

## Context

Followup 04 + custom-answer decision: freeform while awaiting is a custom answer on the
question tool for all harnesses. Esc esc remains cancelled.

## Tasks

- [x] 1. Add `UserChoiceAnswer::Custom { text: String }` in
         `crates/duckchat/src/event.rs` (and any handle API match sites)

- [x] 2. Map Custom in choice encode: Grok `outcome: accepted` with answers map question →
         free text; Claude allow + `updatedInput.answers` question → free text; permission
         wire if needed

- [x] 3. @spec harness/acp-client Mid-turn user choice: Host custom freeform answer completes the pending request

- [x] 4. @spec harness/claude Mid-prompt parent choice: Host custom freeform answer completes with allow and free-text answers

- [x] 5. @spec harness/grok Question wire mapping: Host custom freeform answer completes with an accepted free-text answer
