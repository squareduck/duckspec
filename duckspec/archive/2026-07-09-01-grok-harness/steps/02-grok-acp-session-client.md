# Grok ACP session client

Stand up the grok module and an ACP turn client that spawns `grok agent stdio`, runs the
JSON-RPC handshake, and opens a session — fresh or resumed — reporting the session id.

## Prerequisites

- [ ] @step harness-identity-types

## Context

ACP is JSON-RPC 2.0 over the child's stdio. The turn lifecycle is `initialize` →
(`session/new` when no prior id, else `session/load { sessionId,
cwd, mcpServers }`) →
`session/prompt`. `initialize` advertises `agentCapabilities.loadSession: true` and
`modelState.availableModels`. The child is spawned as `grok agent --always-approve stdio`
so tool permissions never round-trip. Responses are matched by request `id`;
`session/update` messages are notifications. Test lifecycle behavior against a scripted
fake stdio peer that records the request method and params — do not require a live grok
binary.

## Tasks

- [x] 1. Add the `grok` module (`crates/duckchat/src/grok.rs` + `grok/` dir) to `lib.rs`,
         plus any Cargo deps needed for line-delimited JSON-RPC over child stdio.

- [x] 2. Implement `AcpTurn::spawn` (launch `grok agent --always-approve stdio` with the
         working dir) and `initialize`, parsing the handshake result (available models,
         `loadSession`).

- [x] 3. Implement `AcpTurn::open` — `session/new` when no session id is given,
         `session/load` for the given id — returning the resolved session id.

- [x] 4. Implement `AcpTurn::prompt` sending `session/prompt`, and a read loop that
         separates id-matched responses from `session/update` notifications and
         auto-answers any agent→client request.

- [x] 5. @spec harness/grok Session lifecycle and resume: A turn without a prior session opens a new session

- [x] 6. @spec harness/grok Session lifecycle and resume: A turn with a prior session id resumes that session
