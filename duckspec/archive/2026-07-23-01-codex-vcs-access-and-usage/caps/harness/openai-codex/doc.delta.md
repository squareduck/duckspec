# @ OpenAI Codex harness

## ~ Session lifecycle

```
session/new(cwd)  ─┐
                   ├─► refresh repository access ─► remember by thread id
session/load(cwd) ─┘
          │
          ├─ new thread: thread/start
          └─ known thread: thread/resume
                         │
                         ▼
session/prompt ─► turn/start + explicit repository sandbox policy
                         │
                         └─ stream profile updates
```

New conversations open a Codex thread immediately so the id the host persists is the real
thread id from the first open. The agent keeps one app-server child warm across main turns
when possible. Cancel best-effort interrupts a tracked turn and then ends that heat; a
later turn may spawn again and still resume the stored thread id.

Repository access context is separate from app-server process membership. Session open and
load refresh it from the normalized working directory, and every turn applies it
explicitly. Restarting the app-server therefore clears process membership without losing
the repository boundary. Restarting the ACP agent reconstructs the same context from the
next `session/load`.

Mid-prompt cancel from the host usually kills the owned ACP process through the shared
client path rather than waiting for an in-band `session/cancel`.

## ~ Events and tools

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

## + Repository access

Each turn uses workspace-write plus explicit writable roots for existing `.git` and `.jj`
directories directly beneath the normalized repository root:

```
repository
├── working files   workspace-write
├── .git/           additional writable root when present
└── .jj/            additional writable root when present
```

Discovery stays inside that root. The agent does not search ancestors, follow a `.git`
file into an external worktree store, or grant absent metadata paths. A repository without
direct metadata directories receives workspace-write with no additional roots.

This sandbox grants the capability to use Git and Jujutsu. Duckspec workflow and project
instructions such as `AGENTS.md` remain responsible for deciding when commits or
destructive version-control operations are appropriate. A backend that rejects the policy
fails the turn; the agent does not silently retry with a different permission boundary.
