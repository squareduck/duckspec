# Post-implementation review: stuck reply-suggestion loading

Reviewed the change end-to-end (proposal → design → cap deltas → steps → code). The fix
hangs at the right layer, matches the design decisions, and is covered by audit-clean
tests. One scenario’s test models the post-condition without exercising the wire that
produces it — optional tighten, not a block.

## Scope

Post-implementation. Read proposal, design, `caps/chat/default-prompts` and
`caps/harness/warm-runtime` deltas, steps 01–03, and the implementation under
`crates/duckchat` (worker budget/cold-reset, Claude kill-on-drop, Grok hang recovery) and
`crates/duckboard` (`DefaultPromptsReady`, `ProcessExited` clear, `default_prompts`
tests). Scoped audit is clean.

## Findings

### Agent-handle-ends scenario never touches session clear — quality/minor

`crates/duckboard/src/default_prompts.rs` (`agent_handle_ends_while_suggestions_pending`)
asserts chrome for `pending = false` and a heuristic list — the *after* state. It never
calls `AgentSession::clear_agent_default_prompts` or models `ProcessExited`. The real belt
path is the one-liner in `main.rs` on `AgentEvent::ProcessExited`; if that call were
removed, this test would still pass.

The timeout/failure settle test is stronger (it goes through `apply_oneshot_if_current`
with a timeout-shaped `Err`). The handle-end scenario is the belt for “worker gone, no
`DefaultPromptsReady`,” and that path is the one left unasserted.

**Action (optional):** unit-test `begin_default_prompts_oneshot` then
`clear_agent_default_prompts` (pending false, gen advanced) and assert `defaults_chrome`
is not `Loading`; keep the pure chrome check as a companion if useful. Not required for
archive if the one-liner is accepted by inspection.

## What holds up

- **Right boundary.** Budget + cold-reset live on the serial oneshot worker loop
  (`worker.rs`), not a UI-only timer. That is the only place that unsticks later turns.

- **Fidelity.** 10s per Work, aggressive `shutdown` on any `Err`, same duckboard settle
  for timeout as other failures, `ProcessExited` clear — matches design decisions and
  deltas.

- **Claude kill path.** Shared inflight child + `KillInflightOnDrop` fixes the previous
  fire-and-forget thread that could not be killed on timeout.

- **Warm-runtime scenarios** use an injected short budget so CI does not wait the full 10s
  while still proving the budget mechanism and post-failure recovery.

## Verdict

Acceptable as done. The hang class that motivated the change is addressed at the worker
boundary with killable Claude heat, Grok shutdown recovery, and duckboard pending
settlement on failure and process exit. The only finding is a soft test gap on the
ProcessExited belt; it does not erode the design or leave the queue wedged again. Optional
test tighten via `/ds-step` if desired; otherwise archive-ready.
