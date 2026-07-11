# Map Claude thinking to thought chunks

Parse Claude thinking stream deltas and emit profile `agent_thought_chunk` updates on the
live path from step 07 so duckboard Thinking UI can show them like Grok reasoning.

## Prerequisites

- [x] @step stream-profile-updates-during-claude-turns

## Context

Live Claude emits `thinking_delta` / thinking content; protocol `DeltaBlock` only carries
`text` today, so thinking is dropped. Host already maps `agent_thought_chunk` →
`ReasoningDelta`.

## Tasks

- [x] 1. Extend `DeltaBlock` (and related parse) in
         `crates/duckchat-claude-acp/src/claude/protocol.rs` to capture thinking deltas.

- [x] 2. Map thinking deltas to profile `agent_thought_chunk` in
         `crates/duckchat-claude-acp/src/claude/map.rs`.

- [x] 3. @spec harness/claude Profile-compatible event emission: Claude thinking surfaces as profile thought chunks

- [x] 4. Run `duckchat-claude-acp` tests and fix regressions.
