# Effective list is oneshot only

Stop appending the lifecycle heuristic after the oneshot; the effective empty-input list
is exactly the parsed suggestion list (or empty on failure / empty parse).

## Prerequisites

- [x] @step oneshot-framing-with-heuristic-and-order

## Context

Addresses the product change from the post-spec revision and removes merge behavior that
`reviews/01-post-implementation-review.md` no longer wants. Today
`crates/duckboard/src/default_prompts.rs` still implements `merge_default_prompts` /
`effective_prompts(agent, obvious_command)`. Replace with parse-only effective list;
update all call sites that pass `obvious_command` into merge. Keep pure cycle /
empty-submit helpers; re-point unit tests at the new scenarios (delete obsolete merge
tests).

## Tasks

- [x] 1. Replace merge helpers with parse-only effective list (agent suggestions alone; no
         heuristic append); remove or rewrite `merge_default_prompts`

- [x] 2. Update call sites in `interaction.rs` / `main.rs` / view wiring so they no longer
         merge `obvious_command` into the displayed or sent list

- [x] 3. @spec chat/default-prompts Effective list is oneshot result only: Parsed replies are the effective list in order

- [x] 4. @spec chat/default-prompts Effective list is oneshot result only: Failed or empty oneshot yields empty effective list

- [x] 5. @spec chat/default-prompts Empty-input send and cycle: Empty submit sends the active prompt

- [x] 6. @spec chat/default-prompts Empty-input send and cycle: Empty submit is a no-op when the list is empty

- [x] 7. @spec chat/default-prompts Empty-input send and cycle: Tab cycles active index with wrap
