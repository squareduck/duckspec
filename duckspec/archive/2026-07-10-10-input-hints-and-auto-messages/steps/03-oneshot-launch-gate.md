# Oneshot launch gate

Do not start reply-suggestion oneshots when agent input hints are disabled.

## Prerequisites

- [x] @step effective-input-hints-pure

## Tasks

- [x] 1. Add a small pure helper (in `default_prompts` or next to the TurnComplete path)
         that decides whether a reply-suggestion oneshot may start given
         `agent_input_hints` and the existing non-priming / assistant-present conditions

- [x] 2. Gate the `TurnComplete` reply-suggestion spawn in `crates/duckboard/src/main.rs`
         on `state.config.chat.agent_input_hints` (and the helper)

- [x] 3. @spec chat/default-prompts Agent input hints gate: Oneshot launch requires agent input hints enabled
