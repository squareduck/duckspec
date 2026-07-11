# Claude AskUserQuestion bridge

Allow AskUserQuestion on the Claude backend and bridge mid-prompt canUseTool/control
requests to the ACP parent as host user choices, with allow+answers or cancel.

## Prerequisites

- [x] @step acp-mid-turn-user-choice

## Tasks

- [x] 1. Remove `AskUserQuestion` from `DISALLOWED_TOOLS` in
         `crates/duckchat-claude-acp/src/claude/spawn.rs`; keep ordinary-tool
         `bypassPermissions`

- [x] 2. Enable stream-json control / permission-prompt path so AskUserQuestion can reach
         the agent (stdio, not TTY)

- [x] 3. On AskUserQuestion control/tool request: emit profile tool_call; issue parent ACP
         choice; accept parent responses mid-prompt (full-duplex pump)

- [x] 4. Map host selection → allow + `updatedInput` with `questions` and `answers`
         (question text → option label); map cancel → deny/skip without accepting

- [x] 5. Ensure non-question tools under bypass do not emit host user-choice events

- [x] 6. @spec harness/claude AskUserQuestion available: AskUserQuestion is not among disallowed tools

- [x] 7. @spec harness/claude Mid-prompt parent choice: An AskUserQuestion request surfaces a host user choice

- [x] 8. @spec harness/claude Mid-prompt parent choice: Host selection completes with allow and answers

- [x] 9. @spec harness/claude Mid-prompt parent choice: Host cancel completes without accepting the questionnaire

- [x] 10. @spec harness/claude Ordinary tools stay auto-approved: Non-question tools do not require host UI under bypass
