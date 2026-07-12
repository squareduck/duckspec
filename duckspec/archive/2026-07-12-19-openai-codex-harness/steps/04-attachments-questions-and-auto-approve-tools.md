# Attachments, questions, and auto-approve tools

Map ACP prompt blocks to turn input (localImage temps for images), bridge structured
user-input to parent host choices, and auto-allow ordinary tools.

## Prerequisites

- [x] @step profile-event-mapping

## Tasks

- [x] 1. Implement `content.rs`: ACP text/image blocks → App Server text + localImage
         (temp file; cleanup after turn/cancel)

- [x] 2. Implement `ask_user.rs`: tool/requestUserInput ↔ parent
         session/request_permission product options (select / freeform / cancel)

- [x] 3. Auto-complete ordinary allow/reject permission requests without host UI; use
         approvalPolicy never (or equivalent) on thread/turn

- [x] 4. @spec harness/openai-codex Prompt attachments: A resolved image attachment is delivered as a local image input on the turn

- [x] 5. @spec harness/openai-codex Prompt attachments: Surrounding text is preserved as text inputs

- [x] 6. @spec harness/openai-codex Prompt attachments: An unresolved attach marker is left literal

- [x] 7. @spec harness/openai-codex Mid-turn structured questions: A structured user-input request surfaces a host user choice

- [x] 8. @spec harness/openai-codex Mid-turn structured questions: Host selection completes with accepted answers

- [x] 9. @spec harness/openai-codex Mid-turn structured questions: Host custom freeform completes with accepted free-text answers

- [x] 10. @spec harness/openai-codex Mid-turn structured questions: Host cancel completes without accepting the questionnaire

- [x] 11. @spec harness/openai-codex Ordinary tools stay auto-approved: Ordinary tool permission does not require host UI
