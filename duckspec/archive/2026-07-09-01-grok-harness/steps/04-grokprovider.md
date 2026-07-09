# GrokProvider

Implement the `Provider` trait for grok on top of the ACP turn client: model discovery,
title summaries, command discovery, and graceful unavailability.

## Prerequisites

- [ ] @step grok-acp-session-client
- [ ] @step grok-event-translation

## Context

`list_models` sources models and their `context_window` from the ACP handshake's
`modelState.availableModels[].totalContextTokens`, cached after first discovery, each
tagged with the `grok` harness. `title_summary` selects the cheapest available model
(prefer `grok-composer-2.5-fast`, fall back to another available model when absent).
`list_commands` reuses the existing `.claude` command scan — grok loads the same skills.
Capabilities set `reasoning: true`.

Notes from step 02: the handshake parsing already exists — `AcpTurn::initialize` returns
`InitResult { load_session, models: Vec<AcpModel> }`, where `AcpModel { id, name,
context_window }` reads `modelId|id`, `name|displayName`, and `totalContextTokens` from
`modelState.availableModels[]`. `list_models` builds `ModelInfo` from these. The prompt's
`model` and `reasoningEffort` param field names in `AcpTurn::prompt` are **provisional**
(unverified against the live grok ACP) — confirm and correct them here when wiring
`run_turn`. `run_turn` maps `PromptResult.stop_reason` to `TurnComplete` (on `end_turn`)
or `Error`.

Note from step 03: event translation is done and wired behind
`AcpTurn::prompt_events(session_id, text, model, reasoning, context_window,
&events)` — a thin wrapper over `prompt` that runs the pure `map_update`
(`grok/event.rs`) and `try_send`s each neutral `AgentEvent` onto the caller's
channel. `run_turn` should call `prompt_events` (not build its own sink),
passing the selected model's `context_window` (from `list_models`/handshake) so
`UsageUpdate` carries the meter denominator. `prompt_events` still returns
`PromptResult`, so the `stop_reason` → `TurnComplete`/`Error` mapping stays in
`run_turn` as planned. Also emit `SessionIdUpdated` with the id from
`AcpTurn::open` before prompting.

## Tasks

- [x] 1. Define `GrokProvider` and implement `id`, `capabilities`, and `run_turn` (drive
         an `AcpTurn`: spawn, initialize, open, prompt, return the session id).

- [x] 2. Implement `list_models` from the cached handshake, tagging each model with the
         grok harness and its context window.

- [x] 3. Implement `title_summary` selecting the cheapest available model with fallback
         when the preferred fast model is absent.

- [x] 4. Implement `list_commands` by reusing the existing `.claude` scan.

- [x] 5. Make discovery and turns degrade gracefully when the binary or auth is
         unavailable: empty model list, typed turn error, no panic.

- [x] 6. @spec harness/grok Model discovery: Discovered models are tagged with the grok harness and a window

- [x] 7. @spec harness/grok Model discovery: Title model falls back when the preferred fast model is absent

- [x] 8. @spec harness/grok Graceful unavailability: A missing grok binary yields no models and a turn error

## Outcomes

- **`GrokProvider` holds a spawner closure, not `bin: PathBuf`.** The design sketch showed
  `{ bin, models }`, but `grok` is resolved through the login shell (`spawn::grok_command`),
  not an explicit path, so `bin` had no use. Instead the provider carries a
  `Spawner = Arc<dyn Fn() -> Command>` (default = `grok_command`) plus the cached
  `OnceLock<Vec<ModelInfo>>`. A `#[cfg(test)] with_spawn` seam injects a command pointing at
  a missing binary, which is how the graceful-unavailability spec is exercised without
  mutating the environment. `AcpTurn::spawn` was split into `spawn` (real) + `spawn_with(cmd,
  cwd)` (the shared body) to support this.

- **Cancellation was threaded through the ACP turn.** `Provider::run_turn` receives a
  `CancelToken`; `AcpTurn::prompt` / `prompt_events` / the shared `request` loop now take it
  and check it cooperatively between protocol lines (killing the child and returning
  `Error::Cancelled`), matching the Claude path. `initialize`/`open` keep their public
  signatures and pass a non-cancelling token internally, so step 02/03's tests are
  unaffected. Note: a fully-idle read still blocks until the next line arrives (no timer —
  the `tokio` build has no `time` feature), which is the same cooperative semantics the
  `CancelToken` docstring describes.

- **`claude_code::discover_commands` is re-exported** (`pub(crate)`) so `list_commands`
  reuses the identical `.claude` scan rather than duplicating it.
