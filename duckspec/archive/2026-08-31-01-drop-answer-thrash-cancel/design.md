# Drop answer-thrash cancel - Design

Duckboard keeps live-draft replace after thought and deletes the replacement budget, trip
flag, stop notice, and the `handle.cancel()` that followed a trip. Cancel-resync stays,
triggered only by user cancel.

## Session apply

`apply_answer_content_delta` in `crates/duckboard/src/area/interaction.rs` keeps the
replace rule and loses the budget:

```
reasoning delta     →  do not commit pending_text
answer after thought →  discard pending_text, start the new body
contiguous answer   →  append
tool use            →  commit draft, then record the tool
```

Delete `ANSWER_REPLACE_BUDGET`, `ANSWER_THRASH_STOP_NOTICE`, `answer_replace_count`,
`answer_thrash_tripped`, `reset_answer_thrash`, and `on_answer_thrash_trip`. Drop the trip
no-op in `apply_reasoning_content_delta`. Call sites that only reset the counter (tool
use, turn end, new send) drop those calls; flush and commit behavior there is unchanged.

The ContentDelta arm in `crates/duckboard/src/main.rs` no longer watches for a rising trip
edge, does not append a stop notice, and does not `handle.cancel()`. Thought→answer is an
ordinary replace; the agent child keeps running.

Those two session fields are in-memory only, so removing them needs no persisted-session
migration.

## Cancel-resync

Cancellation is user cancel only. `CancelPressed` still calls `capture_unsynced_draft`;
late answer deltas while cancel is in flight still join that draft; the next send still
appends the reminder once.

Thrash is not a cancel cause. There is no capture, notice, or agent kill on over-budget
rewrite, because there is no budget.

A leftover `unsynced_draft` from an older thrash trip may still ride one send. No cleanup
pass.

## Boundary

All of this stays in duckboard session apply. duckchat’s event stream is unchanged. The
path is harness-neutral: Grok is the painful case, not a special case.

Rejected: keep a dead counter; mute later answers without cancelling; raise N; Grok-only
exemption; remove draft replace.
