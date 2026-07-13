# Post-implementation review: Stream-tick CPU

Reviewed `stream-tick-cpu` end-to-end (proposal → design → caps → steps → duckboard).
Implementation matches the design; residual risk is low-cost polish and regression surface
on untested subscription wiring.

## Scope

`duckspec/changes/stream-tick-cpu` (proposal, design, stream-ui + persistence deltas,
steps), `crates/duckboard/src/main.rs` subscription gates, `area/interaction.rs`
`session_needs_stream_tick`, `area/change.rs` row cache. Post-implementation; audit clean;
focused tests green.

## Summary

```
| # | sev | lens | title | → next |
| --- | --- | --- | --- | --- |
| 1 | minor | quality | Subscription wiring untested | ignore |
| 2 | minor | quality | Frozen dots while awaiting | ignore |
```

## Findings

### 1. Subscription wiring untested - quality/minor

**Where:** `crates/duckboard/src/main.rs:5751-5819` (`any_session_needs_stream_tick` /
`any_session_needs_flush_tick`)

**Why:** Specs and tests lock the pure predicate; a future edit that re-gates StreamTick
on bare `is_streaming` would reintroduce the 10 Hz pump without failing `@spec` tests.

**Action:** Optional — thin test or comment-linked assert on the fold helpers if the gate
is ever touched again; not blocking freeze.

### 2. Frozen dots while awaiting - quality/minor

**Where:** `crates/duckboard/src/widget/agent_chat.rs:1412` (indicator while
`is_streaming`); design risk “Frozen dots while awaiting”

**Why:** Idle await correctly stops the tick, but the thinking dots can freeze next to
choice chips — looks hung to some users. Design accepted freeze; hide-while-awaiting was
optional.

**Action:** Optional one-line view gate (`!is_awaiting_user`) if UX complains; ignore for
archive.

## Verdict

**Ready to freeze.** A + C are correctly realized: tick need matches stream-ui, FlushTick
is dirty-only, Changed Files rows rebuild only on set/toggle. Minors are optional polish,
not structural debt.
