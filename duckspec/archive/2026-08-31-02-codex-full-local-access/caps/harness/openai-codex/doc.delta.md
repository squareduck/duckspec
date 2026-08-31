# @ OpenAI Codex harness

## = Repository access

Full local access

## ~ Full local access

Every turn uses Codex's `dangerFullAccess` policy. Commands run without the Codex
filesystem sandbox and with the files, credentials, processes, networks, databases,
sockets, and services available to Duckboard's OS user. This is a machine-level boundary,
not containment within the selected repository.

Thread start and resume remain approval-free. Ordinary app-server approval requests are
still handled automatically for protocol compatibility, while structured questions keep
using the host user-choice path.

Project instructions such as `AGENTS.md` govern the work the agent is expected to perform,
but they do not provide operating-system isolation. The access policy is unconditional; it
is not inherited from the user's Codex configuration or exposed as a Duckboard setting.

If the app-server rejects the policy, the turn fails through the app-server error path.
The agent does not retry with a sandboxed, inherited, or omitted policy.

## @ Session lifecycle

```
session/new(cwd)  ─► thread/start  ─┐
                                    ├─► session/prompt
session/load(cwd) ─► thread/resume ─┘          │
                                               ▼
                                  turn/start + dangerFullAccess
                                               │
                                               └─► stream profile updates
```

New conversations open a Codex thread immediately so the id the host persists is the real
thread id from the first open. The agent keeps one app-server child warm across main turns
when possible. Cancel best-effort interrupts a tracked turn and then ends that heat; a
later turn may spawn again and still resume the stored thread id.

Full local access is attached explicitly to every turn. It is independent of app-server
process heat and requires no repository context to reconstruct after a restart. Mid-prompt
cancel from the host usually kills the owned ACP process through the shared client path
rather than waiting for an in-band `session/cancel`.
