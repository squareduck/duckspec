# Agent crate scaffold and binary discovery

Add workspace crate `duckchat-codex-acp` with an ACP stdio loop shell and binary
resolution matching the Claude agent pattern.

## Tasks

- [x] 1. Add workspace member `crates/duckchat-codex-acp` (bin `duckchat-codex-acp`) and
         empty ACP stdio main loop (initialize/session stubs ok)

- [x] 2. Implement agent binary resolution: `DUCKCHAT_CODEX_ACP` → sibling of current exe
         → PATH

- [x] 3. @spec harness/openai-codex Agent binary discovery: An explicit env override selects the agent binary

- [x] 4. @spec harness/openai-codex Agent binary discovery: When env is unset, a sibling of the running executable is used if present

- [x] 5. @spec harness/openai-codex Agent binary discovery: A missing agent binary fails the turn with a typed error

- [x] 6. @spec harness/openai-codex Owned ACP agent over official Codex: The harness does not require a Node or npm runtime
