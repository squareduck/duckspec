# Flip Claude provider and host cleanup

Point `ClaudeCodeProvider` at the owned ACP agent launch, prove host turns go through the
agent (not in-host stream-json), and remove the cold stream-json client path from
duckchat.

## Prerequisites

- [x] @step claude-duplex-and-profile-emission

## Tasks

- [x] 1. Implement `ClaudeCodeProvider` launch resolving `duckchat-claude-acp` (discovery
         from step 02) and open shared `AcpMainRuntime` / `AcpOneshotRuntime`.

- [x] 2. Keep title/reply prompt helpers and command discovery on the Claude provider;
         oneshots use the shared ACP oneshot path (N=1).

- [x] 3. Delete the in-host Claude stream-json turn driver and cold main/oneshot runtimes
         (`claude_code/run.rs` client path, cold `ClaudeMainRuntime` spawn-per-turn) once
         the provider is ACP-only.

- [x] 4. Move any remaining Claude protocol/spawn code that only the agent needs fully
         into `duckchat-claude-acp` (prefer move over long-term duplication).

- [x] 5. Confirm duckboard harness dispatch still selects `claude-code` / `grok` providers
         unchanged aside from provider internals.

- [x] 6. @spec harness/claude Owned ACP agent over official Claude CLI: A Claude turn is driven through the owned ACP agent process

- [x] 7. @spec harness/claude Owned ACP agent over official Claude CLI: The agent uses the official claude CLI as its backend
