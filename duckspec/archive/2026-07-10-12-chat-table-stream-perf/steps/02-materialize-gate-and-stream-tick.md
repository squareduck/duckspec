# Materialize gate and stream tick

Gate chat UI rebuild behind `chat_ui_dirty` and structural-vs-tick policy: session text
always applies; pure content deltas materialize on `StreamTick`; tools and turn end
materialize immediately.

## Prerequisites

- [x] @step hybrid-layout-arc-cache

## Tasks

- [x] 1. Implement `chat_ui_dirty`, `materialize_chat_ui`, and structural classification
         in `crates/duckboard/src/area/interaction.rs`; gate `AgentEvent` in
         `crates/duckboard/src/main.rs` so pure content/reasoning deltas only dirty while
         streaming, structural events and non-stream paths materialize immediately; extend
         `StreamTick` to materialize dirty streaming sessions (with stick-to-bottom snap
         when appropriate)

- [x] 2. @spec chat/stream-ui Session apply before materialize: Content deltas accumulate on the session without materialization

- [x] 3. @spec chat/stream-ui Session apply before materialize: Reasoning deltas accumulate on the session without materialization

- [x] 4. @spec chat/stream-ui Bounded materialization while streaming: Pure content deltas alone do not materialize the chat UI

- [x] 5. @spec chat/stream-ui Bounded materialization while streaming: Stream UI tick materializes accumulated session answer text into the chat UI

- [x] 6. @spec chat/stream-ui Bounded materialization while streaming: Tool use materializes the chat UI immediately with an Activity row

- [x] 7. @spec chat/stream-ui Bounded materialization while streaming: Turn complete materializes the final answer immediately
