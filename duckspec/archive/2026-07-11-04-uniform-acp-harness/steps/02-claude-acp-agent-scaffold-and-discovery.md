# Claude ACP agent scaffold and discovery

Add the `duckchat-claude-acp` workspace binary with a hand-rolled ACP server loop and
resolve the agent binary via env override, sibling of the executable, then `PATH`.

## Prerequisites

- [x] @step extract-shared-acp-client

## Tasks

- [x] 1. Add workspace member `crates/duckchat-claude-acp` with a binary target
         `duckchat-claude-acp` and wire it in the root `Cargo.toml` members list.

- [x] 2. Implement hand-rolled ACP server stdio loop (`initialize`, `session/new`,
         `session/load`, `session/prompt`, cancel/kill handling) sufficient for a scripted
         peer and for the shared client to complete a turn against it.

- [x] 3. Implement agent binary resolution used by the Claude host launch:
         `DUCKCHAT_CLAUDE_ACP` → sibling of `current_exe()` → `PATH`.

- [x] 4. Document/local-dev expectation: build both `duckchat-claude-acp` and `duckboard`
         into the same `target/` so sibling resolution works.

- [x] 5. @spec harness/claude Agent binary discovery: An explicit env override selects the agent binary

- [x] 6. @spec harness/claude Agent binary discovery: When env is unset, a sibling of the running executable is used if present

- [x] 7. @spec harness/claude Agent binary discovery: A missing agent binary fails the turn with a typed error
