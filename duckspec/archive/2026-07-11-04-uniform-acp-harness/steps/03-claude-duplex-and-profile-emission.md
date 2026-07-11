# Claude duplex and profile emission

Connect the Claude ACP agent to the official `claude` CLI over duplex stream-json, expose
Claude-native session ids, and emit dialect-profile `session/update` notifications for
text and tools.

## Prerequisites

- [x] @step claude-acp-agent-scaffold-and-discovery

## Tasks

- [x] 1. Port Claude spawn/flags knowledge from `claude_code/run.rs` into the agent
         (`login-shell` wrap, stream-json in/out, disallowed tools, autoMemory off,
         permission bypass when required).

- [x] 2. Implement duplex main heat: hold one `claude` process across main turns when
         possible; cancel tears it down; later turns may re-spawn.

- [x] 3. Map ACP `sessionId` to Claude Code's native session id on new/load and surface
         that id to the client.

- [x] 4. Implement Claude protocol → profile `session/update` mapping (assistant text
         chunks; tool call + completed result sharing call id).

- [x] 5. Translate ACP prompt content blocks (text/image) into Claude user content for the
         inner CLI.

- [x] 6. Cover duplex/session/profile with unit or agent-level tests (scripted Claude peer
         or fixtures where live CLI is not required).

- [x] 7. @spec harness/claude Session lifecycle and native session ids: A turn without a prior session opens a new session and surfaces Claude's native session id

- [x] 8. @spec harness/claude Session lifecycle and native session ids: A turn with a prior Claude session id resumes that id

- [x] 9. @spec harness/claude Duplex main heat: A second main turn reuses the inner Claude process when duplex-hot

- [x] 10. @spec harness/claude Duplex main heat: After cancel, a later turn may start Claude again and resume a prior session id

- [x] 11. @spec harness/claude Profile-compatible event emission: Assistant text from Claude surfaces as profile content updates

- [x] 12. @spec harness/claude Profile-compatible event emission: A Claude tool call surfaces as profile tool use then result
