# Kind-aware stream flush

Stop co-mingling reasoning and answer deltas into one buffer: flush by kind on stream
events so committed content never mixes Thinking into Answer.

## Prerequisites

- [ ] @step reasoning-content-model-and-persistence

## Tasks

- [x] 1. Add `flush_pending_reasoning` and use `flush_all_pending` (reasoning then text)
         on tool use and turn complete in `crates/duckboard/src/main.rs`

- [x] 2. On `ReasoningDelta`, flush pending text then append to `pending_reasoning`; on
         `ContentDelta`, flush pending reasoning then append to `pending_text`

- [x] 3. Include `ContentBlock::Reasoning` in `build_history_preamble` in
         `crates/duckboard/src/area/interaction.rs`

- [x] 4. Clear `pending_reasoning` wherever the session resets pending text at turn start
         / error paths
