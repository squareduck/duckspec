# Empty model catalog when backend unavailable

Stop advertising curated models when `codex app-server` cannot be used; honor Graceful
unavailability so a missing or unusable Codex backend yields an empty model list.

## Prerequisites

- [x] @step readme-harness-requirements-and-install-links

## Context

From review finding 1
(`reviews/02-review-post-implementation-review-of-openai-codex-harness.md`).

`resolve_advertised_models` currently maps any live discovery failure to
`curated_fallback()`. With `duckchat-codex-acp` shipped next to duckboard, a machine
without `codex` still populates the openai-codex picker. Investigation: empty advertise on
process/discovery failure and on empty live list (do not keep Claude-style offline aliases
— they are useless without app-server).

## Tasks

- [x] 1. Change `resolve_advertised_models` in `crates/duckchat-codex-acp/src/models.rs`
         so process/discovery failure and empty live list yield an **empty** advertise
         set, not `curated_fallback()`

- [x] 2. Keep or remove `curated_fallback` only as needed after the flip (drop if unused;
         do not reintroduce it as the default failure path)

- [x] 3. Update agent model unit tests: failed/empty live discovery expects empty
         `availableModels` (replace `failed_live_discovery_advertises_curated_fallback`)

- [x] 4. Add “agent up, backend down → no models”: failing app-server spawn factory →
         `initialize` has empty `availableModels`

- [x] 5. @spec harness/openai-codex Graceful unavailability: A missing agent or backend yields no models and a turn error
