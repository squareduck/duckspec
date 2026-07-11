# Duckboard choice wiring

Wire live `UserChoiceRequest` events into the fast-response shell, answer via handle RPC
(not `send_prompt_text`), and clear state on turn end/cancel.

## Prerequisites

- [x] @step rename-shell-to-fast-response
- [x] @step acp-mid-turn-user-choice

## Tasks

- [x] 1. Add `is_awaiting_user` on `AgentSession`; set on `UserChoiceRequest`, clear on
         answer, turn complete, error, process exit, and cancel

- [x] 2. Fill `fast_response` from the event (options + cancel source
         `UserChoice { correlation_id }`); map duckboard agent events through
         `crates/duckboard/src/agent.rs`

- [x] 3. On chip activation for `UserChoice` source: call
         `handle.answer_user_choice(...)`; do not append a user message

- [x] 4. Gate refresh so it does not clear options while awaiting a user choice

- [x] 5. Sweep remaining “obvious chrome” / lifecycle-chip comments in scope orientation
         and chat code to match session/scope prose

- [x] 6. @spec chat/fast-response Visibility: Awaiting user shows chips while turn is open

- [x] 7. @spec chat/fast-response Population: Refresh does not clear options while awaiting a user choice

- [x] 8. @spec chat/fast-response Question activation: Option activation answers the pending choice without a new user message
