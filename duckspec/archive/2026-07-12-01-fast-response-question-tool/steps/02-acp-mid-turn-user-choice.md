# ACP mid-turn user choice

Shared ACP client main-path routing for mid-turn agent→client requests: auto-allow tool
permissions, park and emit neutral user-choice events for structured questions, complete
from host answer or turn cancel, and keep oneshot non-blocking.

## Tasks

- [x] 1. Add `UserChoiceRequest` / `UserChoiceOption` / `UserChoiceAnswer` and
         `AgentEvent::UserChoiceRequest` in `crates/duckchat/src/event.rs`

- [x] 2. Add `AgentCommand::AnswerUserChoice` and `AgentHandle` API to deliver answers to
         a pending oneshot map owned by the worker/turn

- [x] 3. In `AcpTurn::request`, classify agent→client methods: auto-allow
         `session/request_permission` with only allow/reject kinds; park and emit for
         structured questions (`x.ai/ask_user_question` and product-labeled permission
         choices); safe non-blocking completion for unknown methods

- [x] 4. On turn cancel, complete any pending choice as cancelled

- [x] 5. Keep oneshot path from blocking on host UI for choice-shaped requests

- [x] 6. @spec harness/acp-client Mid-turn tool permission auto-allow: Permission request with only allow/reject kinds is auto-allowed

- [x] 7. @spec harness/acp-client Mid-turn user choice: Structured question request surfaces a user-choice event

- [x] 8. @spec harness/acp-client Mid-turn user choice: Host selected answer completes the pending request

- [x] 9. @spec harness/acp-client Mid-turn user choice: Host cancel completes the pending request as cancelled

- [x] 10. @spec harness/acp-client Mid-turn user choice: Turn cancel completes a pending choice as cancelled

- [x] 11. @spec harness/acp-client Headless and oneshot safety: Oneshot path does not block waiting on a host UI choice
