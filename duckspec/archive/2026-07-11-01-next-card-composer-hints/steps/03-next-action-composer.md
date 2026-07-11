# Next-action composer

Build the primary next-action list (empty-session bootstrap vs trailing `next`), session
state, ghost placeholder, empty Enter / Tab cycle, and tab-available marker.

## Prerequisites

- [x] @step meta-card-parser
- [x] @step config-and-obvious-chrome-shell

## Tasks

- [x] 1. Add `next_action_list` (and related helpers) in `default_prompts.rs`; store
         `next_actions` / `next_action_idx` on `AgentSession`; refresh from last assistant
         / lifecycle bootstrap

- [x] 2. Wire empty Enter and Tab/Shift-Tab to next actions only; set empty-input
         placeholder to the active send text; show tab-available marker when `len > 1`

- [x] 3. Ensure empty Enter can still send a next action while a reply-suggestion oneshot
         is pending

- [x] 4. @spec chat/default-prompts Next-action list: Empty session seeds first lifecycle

- [x] 5. @spec chat/default-prompts Next-action list: Empty session without lifecycle yields empty

- [x] 6. @spec chat/default-prompts Next-action list: Non-empty session uses trailing next actions only

- [x] 7. @spec chat/default-prompts Next-action list: Non-empty session without trailing next yields empty

- [x] 8. @spec chat/default-prompts Next-action list: Oneshot results do not enter the next-action list

- [x] 9. @spec chat/default-prompts Next-action empty-input send and cycle: Empty submit sends the active next action

- [x] 10. @spec chat/default-prompts Next-action empty-input send and cycle: Empty submit is a no-op when the next-action list is empty

- [x] 11. @spec chat/default-prompts Next-action empty-input send and cycle: Tab cycles next actions with wrap

- [x] 12. @spec chat/default-prompts Next-action empty-input send and cycle: Multi next shows a tab-available marker

- [x] 13. @spec chat/default-prompts Oneshot readiness: Empty Enter still sends next action while oneshot pending
