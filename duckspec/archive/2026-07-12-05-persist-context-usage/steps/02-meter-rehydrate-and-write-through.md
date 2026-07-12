# Meter rehydrate and write-through

Seed live meter counters from the durable session on load, and write the input+output sum
back onto the session whenever usage updates arrive.

## Prerequisites

- [x] @step durable-context-tokens-field

## Tasks

- [x] 1. In `AgentSession::from_session` (`crates/duckboard/src/area/interaction.rs`), set
         `agent_input_tokens = session.context_tokens` and `agent_output_tokens = 0`

- [x] 2. In the `UsageUpdate` handler (`crates/duckboard/src/main.rs`), after the existing
         `if > 0` merges on live counters, set
         `session.context_tokens = agent_input_tokens + agent_output_tokens`. Do not set
         `needs_flush` for usage alone
