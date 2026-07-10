# LLM-only composer defaults

Composer empty-input defaults come only from a settled non-empty oneshot parse. The
lifecycle heuristic no longer seeds or backs the list.

## Prerequisites

- [x] @step steps-complete-suggests-archive

## Tasks

- [x] 1. In `crates/duckboard/src/default_prompts.rs`, make `effective_prompts` return
         only non-empty oneshot parse strings (drop heuristic fallback argument and path)

- [x] 2. Update `apply_oneshot_if_current` so empty parse and errors yield an empty list
         (still ignore generation mismatch)

- [x] 3. Stop seeding `agent_default_prompts` from the heuristic in
         `refresh_obvious_command` (`area/change.rs`); keep setting `obvious_command` /
         `scope_facts`

- [x] 4. Drop `obvious_command` arguments from call sites that build the effective list or
         empty-submit path (`area/interaction.rs` and any other callers)

- [x] 5. @spec chat/default-prompts Effective default-prompt list: No non-empty oneshot result yields an empty list

- [x] 6. @spec chat/default-prompts Effective default-prompt list: Failed or empty oneshot yields an empty list even with a heuristic

- [x] 7. @spec chat/default-prompts Suggestion readiness: Timed-out or failed oneshot settles to ready
