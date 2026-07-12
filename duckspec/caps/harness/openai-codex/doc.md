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
session/new  →  thread/start  →  sessionId = thread.id
session/load →  thread/resume (missing → session-not-found)
session/prompt → turn/start   → stream profile updates
session/cancel → turn/interrupt (if in flight) + kill app-server heat
```

New conversations open a Codex thread immediately so the id the host persists is the real
thread id from the first open. The agent keeps one app-server child warm across main turns
when possible. Cancel best-effort interrupts a tracked turn then ends that heat; a later
turn may spawn again and still resume the stored thread id. Mid-prompt cancel from the
host usually kills the owned ACP process (shared client pattern) rather than waiting for
an in-band `session/cancel`.

## Events and tools

The agent maps Codex item and usage streams into the shared ACP profile (assistant text,
tool use/result pairs, token totals). Ordinary tool permission prompts are auto-allowed so
the host UI is not required for every shell or file action. Structured user-input requests
become host user-choice chips (selection, freeform, or cancel).

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
