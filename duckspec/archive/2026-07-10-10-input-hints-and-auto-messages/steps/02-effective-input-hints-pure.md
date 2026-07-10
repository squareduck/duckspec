# Effective input hints pure

Expand pure `effective_prompts` for empty-session lifecycle seed and agent-gated oneshot
lists; cover all effective-list scenarios with unit tests.

## Prerequisites

- [x] @step chat-config-flags

## Tasks

- [x] 1. In `crates/duckboard/src/default_prompts.rs`, expand `effective_prompts` to take
         `session_empty`, `first_lifecycle`, `oneshot_replies`, and `agent_input_hints`
         per the design; empty session returns formatted lifecycle\[0\] only; non-empty
         uses oneshot only when agent hints are on

- [x] 2. Update existing unit tests and any pure call sites of the old signature so the
         crate compiles with the new rules

- [x] 3. @spec chat/default-prompts Effective default-prompt list: Parsed replies are the effective list in order

- [x] 4. @spec chat/default-prompts Effective default-prompt list: No non-empty oneshot result yields an empty list

- [x] 5. @spec chat/default-prompts Effective default-prompt list: Failed or empty oneshot yields an empty list even with a heuristic

- [x] 6. @spec chat/default-prompts Effective default-prompt list: Empty session seeds first lifecycle

- [x] 7. @spec chat/default-prompts Effective default-prompt list: Empty session without lifecycle yields empty

- [x] 8. @spec chat/default-prompts Effective default-prompt list: Non-empty session with agent hints disabled yields empty despite oneshot

- [x] 9. @spec chat/default-prompts Effective default-prompt list: Empty session ignores oneshot results
