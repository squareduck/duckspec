# Claude progressive stream and thinking

User-led followup after the hang fix: Claude answers in chat but only after the full turn;
Grok streams. Root cause is the agent batching `session/update` until prompt completion.
Also map Claude thinking deltas onto the existing thought channel for parity with Grok UX.

## Scope

Post steps 05–06 on `uniform-acp-harness`. Discussed agent `main.rs` prompt path (collect
updates, then notify), shared client live `map_update`, and Claude wire vs agent map
(`text_delta` yes; `thinking_delta` dropped). Host already supports `agent_thought_chunk`
→ `ReasoningDelta` → Thinking UI.

## Summary

```
| # | sev | lens | title | → next |
| --- | --- | --- | --- | --- |
| 1 | major | quality | Claude agent batches session/update until turn end | /ds-step |
| 2 | minor | quality | Claude thinking deltas not mapped to agent_thought_chunk | /ds-spec |
```

## Issues

### 1. Claude agent batches session/update until turn end - quality/major

**Where:** `crates/duckchat-claude-acp/src/main.rs` (`session/prompt` waits for
`run_prompt`, then flushes all updates); duplex/agent accumulate `Vec` of updates instead
of emitting live.

**Why:** Answer text only appears when the turn completes — “nothing, then boom full
message.” Breaks progressive UX Grok already has on the same client.

**Action:** Stream each profile `session/update` to the ACP parent as Claude lines arrive;
send `session/prompt` result only at stop. Cover with a test that sees an update before
the prompt result. Prefer `/ds-step` (behavior implied by existing profile emission; add
`/ds-spec` only if a streaming-normative sentence is missing).

### 2. Claude thinking deltas not mapped to agent_thought_chunk - quality/minor

**Where:** `crates/duckchat-claude-acp/src/claude/protocol.rs` (`DeltaBlock` text-only);
`map.rs` only maps `content_block_delta` with `text` to `agent_message_chunk`.

**Why:** Live Claude emits `thinking_delta` / thinking blocks; host and duckboard already
render `agent_thought_chunk` as Thinking. Leaving them unmapped hides reasoning that Grok
already streams.

**Action:** Parse thinking deltas; emit `agent_thought_chunk`. Spec under `harness/claude`
profile emission if not already covered, then implement with streaming so thoughts also
appear progressively.

## Outcome

Agreed both progressive text and thinking mapping are in scope. Plan/code unchanged in
this write. Not archive-ready until progressive streaming ships at least (#1); #2 is the
same UX pass if taken together.
