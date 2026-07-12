# Claude host model discovery

Make `ClaudeCodeProvider::list_models` discover models from the owned agent’s ACP
initialize result (remove the host static table), with display names, optional windows,
and empty list on failure.

## Prerequisites

- [x] @step claude-agent-live-catalog

## Tasks

- [x] 1. Replace the static `list_models` vec in `crates/duckchat/src/claude_code.rs` with
         ACP initialize discovery (same handshake pattern as Grok: dedicated thread, memo,
         empty on failure)

- [x] 2. Map each `AcpModel` to `ModelInfo` with harness `claude-code`, provider-local
         `humanize_display`, and `context_window` when advertised

- [x] 3. @spec harness/claude Model discovery: Listed models come from the agent advertise set

- [x] 4. @spec harness/claude Model discovery: Each listed model carries a display name

- [x] 5. @spec harness/claude Model discovery: A model with a known context window carries that window

- [x] 6. @spec harness/claude Model discovery: Discovery failure yields an empty host list without panic
