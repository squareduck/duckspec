# Merge and empty-input selection

Pure duckboard helpers for merge/dedupe, active-index cycling, and empty-submit selection,
with unit tests for every merge and empty-input scenario.

## Prerequisites

- [x] @step reply-parse-and-provider-oneshot

## Tasks

- [x] 1. Add `crates/duckboard/src/default_prompts.rs` with `prompt_key`,
         `merge_default_prompts` / `effective_prompts`, and pure helpers for
         cycle-with-wrap and empty-submit selection (active entry or no-op)

- [x] 2. Wire the module into `duckboard` (`mod` / `use` as needed)

- [x] 3. Unit-tested post-merge heuristic append/dedupe (removed from the capability spec;
         superseded by step 05 oneshot-only effective list)

- [x] 4. @spec chat/default-prompts Empty-input send and cycle: Empty submit sends the active prompt

- [x] 5. @spec chat/default-prompts Empty-input send and cycle: Empty submit is a no-op when the list is empty

- [x] 6. @spec chat/default-prompts Empty-input send and cycle: Tab cycles active index with wrap
