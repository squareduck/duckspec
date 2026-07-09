# Harness dispatch

Route each turn to the provider named by its model's harness, aggregate models across
harnesses, and key the chat subscription on harness.

## Prerequisites

- [ ] @step grokprovider
- [ ] @step model-persistence-and-default

## Context

Three sites construct `ClaudeCodeProvider` directly: `crates/duckboard/src/agent.rs:22`
(`available_models`), `agent.rs:86` (`agent_stream` worker spawn), and
`crates/duckboard/src/main.rs:1598` (title summary). Each becomes a match on the harness.
`spawn_worker<P>` is monomorphized per match arm, so no trait-object change is needed.
`agent_subscription` / `agent_stream` gain a `harness` parameter and the subscription key
includes it so switching harness respawns the worker. `agent.rs` also currently drops
`ReasoningDelta` (`agent.rs` maps it to `continue`) — map it through for the grok path.

Note from step 04: `GrokProvider` lives at `duckchat::grok::GrokProvider` (mirrors
`duckchat::claude_code::ClaudeCodeProvider`); construct it with `GrokProvider::new()`. Its
`list_models()` is synchronous but discovers grok's models by spawning `grok agent stdio`
and running the `initialize` handshake on a one-shot background thread, caching the result
for that provider instance's lifetime. So a fresh `GrokProvider::new()` per
`available_models()` call re-spawns grok every time — for task 2, build the grok provider
once and reuse it (or otherwise memoize) rather than constructing a new one on each
aggregation, to avoid a subprocess spawn on every model-list read. When grok is absent the
list is simply empty, so aggregation stays panic-free with only the Claude models.

Note from step 05: the send-time cascade now resolves an unpinned turn to the built-in
default `grok`/`grok-4.5` (`resolve_turn_model` in `interaction.rs`) and stamps it on
`req.model`. Until this step routes by harness, that model string is still handed to the
Claude worker, so an unpinned turn currently sends `grok-4.5` to Claude. Wiring dispatch
here (keying the worker on the resolved `ModelRef`'s harness) is what makes the default
actually run on grok — treat it as a correctness requirement, not just an enhancement.

## Tasks

- [x] 1. Add a harness→provider dispatch helper and use it in `agent_stream` to spawn the
         matching provider; map `ReasoningDelta` through instead of dropping it.

- [x] 2. Aggregate `available_models` across the Claude and grok providers.

- [x] 3. Dispatch the title-summary site (`main.rs:1598`) on the session's harness.

- [x] 4. Add a `harness` parameter to `agent_subscription`/`agent_stream` and include it
         in the subscription key so a harness switch respawns the worker.

- [x] 5. @spec harness/selection Harness dispatch: A model's harness selects the provider that runs its turn

- [x] 6. @spec harness/selection Harness dispatch: The offered models span every registered harness
