# Grok event translation

Translate grok's ACP `session/update` stream into duckchat's neutral `AgentEvent`s, and
wire the mapping into the prompt read loop.

## Prerequisites

- [ ] @step grok-acp-session-client

## Context

Keep the mapping a pure function (e.g.
`fn map_update(update, model_window) ->
Option<AgentEvent>`) so it is unit-testable from
recorded JSON without a live grok. Mapping: `agent_message_chunk` → `ContentDelta`;
`agent_thought_chunk` → `ReasoningDelta`; `tool_call` →
`ToolUse { id: toolCallId, name: title, input:
rawInput }`; `tool_call_update` (completed)
→ `ToolResult { id: toolCallId, name,
output: content }`; `params._meta.totalTokens` + the
active model's context window → `UsageUpdate`. The `context_window` comes from the model
discovered in the handshake, not from the update.

Note from step 02: `AcpTurn::prompt` already carries the read loop, but it takes a
notification sink `on_update: &mut dyn FnMut(&Value)` (the raw `session/update` `params`)
and returns `PromptResult { stop_reason: Option<String> }` — it does **not** own an
`events` channel. So task 3 wires `map_update` by passing a closure that translates each
`params` and forwards the result onto the caller's `mpsc::Sender<AgentEvent>`; the
`stop_reason` maps to `TurnComplete`/`Error` at the `run_turn` layer (step 04). The
totalTokens live at `params._meta.totalTokens` in the update.

## Tasks

- [x] 1. Implement the pure `map_update` function covering content, reasoning, tool-call,
         tool-result, and usage variants.

- [x] 2. Fold the active model's context window into the emitted `UsageUpdate`.

- [x] 3. Wire `map_update` into `AcpTurn::prompt`'s read loop so each translated event is
         sent on the `events` channel.

- [x] 4. @spec harness/grok Event translation: Assistant text and reasoning surface on distinct channels

- [x] 5. @spec harness/grok Event translation: A tool call surfaces as a use then a matching result

- [x] 6. @spec harness/grok Event translation: A usage update carries used tokens and the model's context window
