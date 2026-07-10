# Pre-oneshot heuristic defaults

Seed and fall back empty-input defaults from the lifecycle heuristic so new sessions and
failed oneshots still arm empty Enter. Local to duckboard — no agent runtime.

## Prerequisites

- [x] @step duckboard-handle-dispatch

## Tasks

- [x] 1. Extend `crates/duckboard/src/default_prompts.rs`: `heuristic_as_prompts`, widen
         `effective_prompts` / `apply_oneshot_if_current` so non-empty oneshot parse wins
         and otherwise the heuristic (empty-send form, leading `/`) is the single-entry
         list.

- [x] 2. Seed `agent_default_prompts` from `obvious_command` when a session is created or
         the heuristic is refreshed and no non-empty oneshot list is armed; keep
         suggestions ready (not pending) for that seed.

- [x] 3. On `DefaultPromptsReady`, apply oneshot results with heuristic fallback on
         failure/empty parse; leave superseded generations unchanged.

- [x] 4. @spec chat/default-prompts Effective default-prompt list: Parsed replies are the effective list in order

- [x] 5. @spec chat/default-prompts Effective default-prompt list: Pre-oneshot list is the lifecycle heuristic when present

- [x] 6. @spec chat/default-prompts Effective default-prompt list: Failed or empty oneshot falls back to the heuristic

- [x] 7. @spec chat/default-prompts Effective default-prompt list: No oneshot and no heuristic yields an empty list

## Outcomes

- Fixed `spec.delta.md` rename target to include the full heading
  (`Requirement: Effective default-prompt list`); without the `Requirement: `
  prefix, merge failed and step `@spec` tasks could not resolve.
