# OpenAI Codex harness

The OpenAI Codex harness lets duckboard drive OpenAI Codex as an agent backend through an
owned ACP agent process over the official `codex app-server`. The host keeps the shared
ACP client; Codex-specific launch, models, attachments, questions, and skill discovery
live in this harness.

## Role

```
Host ACP client
   │  profile ACP only
   ▼
duckchat-codex-acp   (owned agent binary)
   │  App Server JSON-RPC
   ▼
codex app-server     (official Codex)
```

Selecting an openai-codex model only changes which provider and agent process run the
turn. Resume stays harness-bound: a Codex thread id cannot be loaded on Claude or Grok.

## Session lifecycle

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

## Events and tools

The agent maps Codex item and usage streams into the shared ACP profile:

```
Codex stream                         ACP profile
───────────────────────────────────  ───────────────────────────
assistant text                       assistant message chunks
reasoning text                       thought chunks
tool start + completion              tool use/result pair
latest-turn total tokens             usage total
cumulative total (latest absent)     usage compatibility fallback
```

Ordinary tool permission prompts are auto-allowed so the host UI is not required for every
shell or file action. Structured user-input requests become host user-choice chips
(selection, freeform, or cancel).

Token usage describes the latest turn when Codex supplies that snapshot; cumulative thread
consumption is only a compatibility fallback. The shared client remains responsible for
pairing the normalized total with the selected model's context window.

## Models and oneshots

Models come from the agent’s initialize advertise set, each tagged `openai-codex`, with a
display name when known. Title and reply oneshots prefer the harness oneshot model when it
is advertised and fall back to another advertised model otherwise.

## Attachments

User prompts may include `[label](attach:<id>)` markers. Resolved images are delivered as
local image inputs on the Codex turn (temp file for the turn); surrounding text stays as
text inputs. Unresolved markers stay literal text.

## Skills

Composer slash commands for this harness are discovered from `.agents/skills/*/SKILL.md`
under the project root — the same layout `ds init codex` installs. Missing skill trees
yield an empty command list.

## Unavailability

The Codex backend is optional. When the owned agent binary, official `codex`, or auth is
unavailable, model discovery is empty and turns fail with typed errors rather than
panicking, so other harnesses keep working.

## Full local access

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
