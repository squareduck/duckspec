# Post-implementation review confirm-gate-reliability

Post-implementation review of both legs (cancelled-draft resync in duckboard, spec
template reshape in duckspec). Verdict: well-conceived and well-made; three minor
findings, none blocking.

## Scope

Full chain: proposal, design, `caps/chat/cancel-resync` (spec + doc),
`caps/chat/persistence` delta, all three steps, and the touched source —
`crates/duckboard/src/chat_store.rs`, `crates/duckboard/src/area/interaction.rs`,
`crates/duckboard/src/main.rs`, `crates/duckspec/content/templates/spec.md`. Deepest
layer: code.

## Summary

```
| # | sev | lens | title | → next |
| --- | --- | --- | --- | --- |
| 1 | minor | fidelity | Late deltas after user cancel widen the transcript gap | /ds-step |
| 2 | minor | soundness | Recovery resend bypasses the reminder, leaves a stale draft | /ds-step |
| 3 | minor | fidelity | Spec norm overclaims "start of the outgoing prompt" | /ds-spec |
```

## Findings

### 1. Late deltas after user cancel widen the transcript gap - fidelity/minor

**Where:** `crates/duckboard/src/area/interaction.rs` (`CancelPressed` capture) vs
`crates/duckboard/src/main.rs:1566` (`TurnComplete` flush)

**Why:** Capture happens at cancel-press, but unlike the thrash path (where
`answer_thrash_tripped` gates further deltas), user cancel keeps applying answer deltas
until the agent actually stops; `TurnComplete` then flushes them into the visible
transcript. The user can see more text than the captured draft, so the resync reminder
under-reports what they responded to — a small, real erosion of the invariant the
capability exists to hold.

**Action:** Mark cancel-in-flight on `CancelPressed` and re-capture at that turn's
`TurnComplete` before the flush, or gate deltas after user cancel the same way the thrash
trip does.

### 2. Recovery resend bypasses the reminder, leaves a stale draft - soundness/minor

**Where:** resume-loss re-dispatch, `crates/duckboard/src/area/interaction.rs:2902-2913`

**Why:** The recovery path sends directly without `apply_resync_reminder`. Divergence is
covered there — `build_history_preamble` already carries the kept draft as a committed
message — but `unsynced_draft` stays set and rides the next normal send as a duplicate,
now-out-of-context reminder.

**Action:** Clear (`take()`) the unsynced draft in the recovery path; the history preamble
has implicitly synced it.

### 3. Spec norm overclaims "start of the outgoing prompt" - fidelity/minor

**Where:** `caps/chat/cancel-resync/spec.md`, Resync reminder on next send

**Why:** "The user's text SHALL remain at the start of the outgoing prompt" is falsified
by pre-existing prompt composition — selection-context attachments and the legacy history
preamble both prepend to the prompt. The contract the implementation (and its tests)
actually holds is that the user's text precedes the reminder.

**Action:** Soften the norm to "the user's text SHALL precede the reminder" and drop the
absolute claim about the prompt start.

## Verdict

Solid at every layer: the proposal names a verified root cause, the design's capture rule
is grounded in observed harness behavior, specs are falsifiable and covered 7/7, and both
legs are implemented faithfully with tests. The three findings are low-cost touch-ups —
two small code refinements on edge paths and one spec-wording fix — recommended before
archive but not blocking.
