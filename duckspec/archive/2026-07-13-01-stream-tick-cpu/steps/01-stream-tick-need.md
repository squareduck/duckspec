# Stream tick need

Add a pure stream-UI-tick need predicate, subscribe StreamTick and FlushTick only when
needed, and cover the stream-ui tick-need scenarios.

## Tasks

- [x] 1. Add `session_needs_stream_tick` next to `should_materialize_on_stream_tick` in
         `crates/duckboard/src/area/interaction.rs` per design (streaming + not awaiting →
         true; awaiting → only when materialize owed on stick-to-bottom)

- [x] 2. In `crates/duckboard/src/main.rs` `subscription`: replace `any_session_streaming`
         gate — StreamTick when any session needs stream tick; FlushTick when any session
         has `needs_flush`; drop or narrow `any_session_streaming` if unused

- [x] 3. @spec chat/stream-ui Stream UI tick need: Active streaming without awaiting needs the stream UI tick

- [x] 4. @spec chat/stream-ui Stream UI tick need: Idle awaiting without deferred materialize does not need the stream UI tick

- [x] 5. @spec chat/stream-ui Stream UI tick need: Awaiting with deferred pure content on stick-to-bottom needs the stream UI tick

- [x] 6. Run focused duckboard tests for the new unit tests and fix failures
