# App-server client, sessions, and process heat

Spawn official `codex app-server`, drive session new/load/prompt/cancel with thread ids as
ACP session ids, and keep the app-server process warm across main turns until cancel.

## Prerequisites

- [x] @step agent-crate-scaffold-and-binary-discovery

## Tasks

- [x] 1. Implement `codex/` spawn + App Server JSON-RPC client over child stdio
         (initialize, model/list, thread/start|resume, turn/start|interrupt)

- [x] 2. Wire session/new → thread/start and session/load → thread/resume; surface
         thread.id as sessionId; map missing load to session-not-found shape

- [x] 3. Keep one app-server child process-hot across main turns; cancel interrupts and
         kills heat

- [x] 4. @spec harness/openai-codex Owned ACP agent over official Codex: The agent uses official codex app-server as its backend

- [x] 5. @spec harness/openai-codex Session lifecycle and thread ids: A turn without a prior session opens a new session and surfaces a Codex thread id

- [x] 6. @spec harness/openai-codex Session lifecycle and thread ids: A turn with a prior session id resumes that id

- [x] 7. @spec harness/openai-codex Session lifecycle and thread ids: A failed load of a missing session surfaces session-not-found

- [x] 8. @spec harness/openai-codex App-server process heat: A second main turn reuses the app-server process when hot

- [x] 9. @spec harness/openai-codex App-server process heat: After cancel, a later turn may spawn again and resume a prior session id
