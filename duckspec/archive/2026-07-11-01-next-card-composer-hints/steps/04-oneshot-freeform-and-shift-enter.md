# Oneshot freeform and Shift-Enter

Rewrite reply-suggestion oneshot as a single freeform under-input suggestion sent only
with empty Shift-Enter; full message context; no lifecycle heuristic.

## Prerequisites

- [x] @step next-action-composer

## Tasks

- [x] 1. In `duckchat` `reply_suggest`: cap at one `REPLY:`, rewrite instruction to
         freeform user reply, embed full user/assistant bodies, drop lifecycle heuristic
         and command-list priming, remove line truncation helpers/tests

- [x] 2. Gate oneshot launch on `agent_input_hints` only; under-input shows loading/single
         row with ⇧↩ marker (not a multi list)

- [x] 3. Empty Shift-Enter sends armed oneshot when ready; empty Enter never sends
         oneshot; update TextEdit/app key handling as needed

- [x] 4. @spec chat/default-prompts Parsed suggestion list: REPLY lines capped at one

- [x] 5. @spec chat/default-prompts Parsed suggestion list: No matching lines yields an empty list

- [x] 6. @spec chat/default-prompts Parsed suggestion list: Unknown slash text is preserved

- [x] 7. @spec chat/default-prompts Parsed suggestion list: Reply longer than 100 characters is preserved in full

- [x] 8. @spec chat/default-prompts Oneshot request framing: Full assistant and user messages are embedded without line truncation

- [x] 9. @spec chat/default-prompts Oneshot request framing: Lifecycle heuristic is not included in the request

- [x] 10. @spec chat/default-prompts Oneshot request framing: Instruction asks for a freeform user reply and at most one REPLY line

- [x] 11. @spec chat/default-prompts Oneshot request framing: Empty assistant yields empty list without a model call

- [x] 12. @spec chat/default-prompts Oneshot readiness: Pending hides oneshot row and shows loading

- [x] 13. @spec chat/default-prompts Oneshot readiness: Empty Cmd-Enter is a no-op while oneshot pending

- [x] 14. @spec chat/default-prompts Oneshot readiness: Ready after settle arms the oneshot row

- [x] 15. @spec chat/default-prompts Oneshot readiness: Superseded generation does not arm oneshot

- [x] 16. @spec chat/default-prompts Oneshot readiness: Main turn in progress hides oneshot chrome

- [x] 17. @spec chat/default-prompts Oneshot readiness: Timed-out or failed oneshot settles to ready empty

- [x] 18. @spec chat/default-prompts Oneshot readiness: Agent handle ends while oneshot pending becomes ready

- [x] 19. @spec chat/default-prompts Oneshot empty-input send: Empty Cmd-Enter sends the armed oneshot suggestion

- [x] 20. @spec chat/default-prompts Oneshot empty-input send: Empty Cmd-Enter is a no-op when no oneshot suggestion

- [x] 21. @spec chat/default-prompts Oneshot empty-input send: Empty Enter does not send the oneshot suggestion

- [x] 22. @spec chat/default-prompts Oneshot presentation: Armed oneshot shows a Cmd-Enter marker before the suggestion
