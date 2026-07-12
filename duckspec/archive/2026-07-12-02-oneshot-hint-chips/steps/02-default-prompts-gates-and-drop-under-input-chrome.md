# Default-prompts gates and drop under-input chrome

Add oneshot chip eligibility and launch-skip-when-ghost pure helpers; remove under-input
oneshot chrome/send helpers; update readiness tests so pending never shows loading chrome.

## Prerequisites

- [x] @step reply-oneshot-parse-and-instruction

## Context

Under-input oneshot UI was removed in this step (not deferred to step 04) so helpers could
be deleted without leaving a broken compile. Step 04 still owns renames, cap delete, and
chip sync wiring.

## Tasks

- [x] 1. In `crates/duckboard/src/default_prompts.rs`, change `oneshot_display_prompts` to
         keep up to 3 entries (still gated by `agent_input_hints`)

- [x] 2. Add `oneshot_chips_allowed` (streaming, awaiting, next_actions_len, hints,
         oneshot_len)

- [x] 3. Extend launch decision so a non-empty next-action list skips starting the oneshot
         (`should_begin_reply_oneshot` or call-site helper)

- [x] 4. Remove `DefaultsChrome`, `defaults_chrome`, `oneshot_cmd_submit_text`,
         `ONESHOT_CMD_ENTER_MARKER`, and under-input presentation helpers/tests that
         depend on them

- [x] 5. Update readiness helpers/tests: failed settle and handle-end leave ready empty;
         pending presents no loading/row chrome

- [x] 6. @spec chat/default-prompts Oneshot readiness: Failed or timed-out oneshot settles without presenting suggestions

- [x] 7. @spec chat/default-prompts Oneshot readiness: Agent handle end while pending leaves suggestions ready empty

- [x] 8. @spec chat/default-prompts Oneshot readiness: Pending oneshot presents no loading chrome

- [x] 9. @spec chat/default-prompts Agent input hints gate: Oneshot launch is skipped when the next-action list is non-empty

- [x] 10. @spec chat/default-prompts Oneshot chip eligibility: Eligible when idle with no next actions and a settled list

- [x] 11. @spec chat/default-prompts Oneshot chip eligibility: Ineligible when next-action list is non-empty

- [x] 12. @spec chat/default-prompts Oneshot chip eligibility: Ineligible while awaiting a user choice

- [x] 13. @spec chat/default-prompts Oneshot chip eligibility: Ineligible while streaming

- [x] 14. @spec chat/default-prompts Oneshot chip eligibility: Ineligible when the settled list is empty

- [x] 15. Delete or rewrite obsolete under-input scenarios (Cmd-Enter send, presentation
          marker, loading row) so unit tests match the delta removals

- [x] 16. Run `cargo test -p duckchat -p duckboard -- default_prompts reply_suggest` (or
          equivalent) for this step’s coverage
