# Oneshot fast-response shell and activation

Add `OneshotHints` as a shell source, sync settled oneshot replies into options when
eligible, preserve fill across refresh, and activate via normal user-message send.

## Prerequisites

- [x] @step default-prompts-gates-and-drop-under-input-chrome

## Tasks

- [x] 1. In `crates/duckboard/src/fast_response.rs`, add
         `FastResponseSource::OneshotHints` and `from_oneshot_hints` (id == label == reply
         text)

- [x] 2. Rename `bubble_send_text` → `lifecycle_send_text` and update call sites

- [x] 3. In `crates/duckboard/src/area/interaction.rs`, add `sync_oneshot_chips`: leave
         UserChoice alone; fill from settled list when eligible; clear OneshotHints/None
         otherwise

- [x] 4. Change `refresh_fast_response` / product refresh path to re-sync oneshot instead
         of always emptying the shell when not awaiting

- [x] 5. Extend `activate_fast_response` so `OneshotHints` sends option text via
         `send_prompt_text` and clears oneshot list + shell

- [x] 6. @spec chat/fast-response Population: Ordinary refresh leaves options empty when oneshot is ineligible

- [x] 7. @spec chat/fast-response Population: Refresh preserves oneshot fill when still eligible

- [x] 8. @spec chat/fast-response Population: Settled eligible oneshot fills the option shell

- [x] 9. @spec chat/fast-response Population: Live user choice overwrites oneshot fill

- [x] 10. @spec chat/fast-response Population: Oneshot settle does not replace a live user-choice fill

- [x] 11. @spec chat/fast-response Oneshot activation: Option activation sends the oneshot text as a user message
