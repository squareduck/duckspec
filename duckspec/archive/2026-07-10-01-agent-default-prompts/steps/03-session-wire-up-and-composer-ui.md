# Session wire-up and composer UI

Connect oneshot results to ephemeral session state, fire on TurnComplete, and drive empty
Enter plus Tab/Shift-Tab and the multi-option empty-input list from the effective prompts.

## Prerequisites

- [x] @step merge-and-empty-input-selection

## Tasks

- [x] 1. Add ephemeral fields on `AgentSession` (`agent_default_prompts`,
         `default_prompts_gen`, `default_prompt_idx`) and clear agent prompts + bump gen
         when a new turn starts / streams

- [x] 2. On non-priming `TurnComplete`, extract last assistant + optional last user text,
         spawn harness `reply_suggestions` with available slash names, handle
         `DefaultPromptsReady` with gen matching

- [x] 3. Replace empty-Enter `obvious_command` path with active entry from
         `effective_prompts` (no-op when list empty)

- [x] 4. Handle Tab / Shift-Tab for cycle-with-wrap when input is empty and completion
         popup is not consuming Tab (input stays empty)

- [x] 5. Render the full effective list when the composer is empty (active marker; hide
         when input non-empty); drop single-command-only placeholder behavior
