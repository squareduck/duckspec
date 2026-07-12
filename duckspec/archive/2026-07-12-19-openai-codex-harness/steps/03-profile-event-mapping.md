# Profile event mapping

Map Codex App Server item and usage streams into profile `session/update` notifications
the shared ACP client already understands.

## Prerequisites

- [x] @step app-server-client-sessions-and-process-heat

## Tasks

- [x] 1. Implement `codex/map.rs`: agent message → `agent_message_chunk`; tool items →
         tool_call + completed tool_call_update; token usage → `_meta.totalTokens`

- [x] 2. Stream profile updates to the parent during the turn (not only at end)

- [x] 3. @spec harness/openai-codex Profile-compatible event emission: Assistant text surfaces as profile content updates

- [x] 4. @spec harness/openai-codex Profile-compatible event emission: A tool call surfaces as profile tool use then completed result

- [x] 5. @spec harness/openai-codex Profile-compatible event emission: Token telemetry surfaces as usage with total tokens
