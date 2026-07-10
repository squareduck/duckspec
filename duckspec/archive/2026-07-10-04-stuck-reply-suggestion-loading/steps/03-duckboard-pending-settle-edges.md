# Duckboard pending settle edges

Ensure reply-suggestion pending ends on oneshot failure (including timeout-shaped Err) and
when the agent handle exits, so empty-input chrome cannot stick on the loading indicator.

## Prerequisites

- [x] @step worker-oneshot-budget-and-cold-reset

## Tasks

- [x] 1. Confirm `DefaultPromptsReady` with matching gen maps `Err` (including timeout)
         through `apply_oneshot_if_current` to ready + heuristic and clears
         `default_prompts_pending` in `crates/duckboard/src/main.rs`

- [x] 2. On `AgentEvent::ProcessExited`, call `clear_agent_default_prompts()` (or
         equivalent) so pending cannot remain true after the worker is gone

- [x] 3. Add pure/unit coverage for timeout-or-failure settle → ready chrome and heuristic
         list next to existing `default_prompts` tests

- [x] 4. @spec chat/default-prompts Suggestion readiness: Timed-out or failed oneshot settles to ready

- [x] 5. @spec chat/default-prompts Suggestion readiness: Agent handle ends while suggestions pending
