# Warm agent runtimes

Keep agent child processes warm per chat so main turns, title summaries, and reply
suggestions skip spawn and handshake cost. Harness-agnostic runtimes live on the chat’s
agent handle; oneshots use a fresh logical session every call (N=1). Before any oneshot
has armed a list, empty-input defaults use the lifecycle heuristic; a failed or empty
oneshot falls back to that heuristic.

## Motivation

Reply suggestions (and title summaries) feel slow because each call cold-spawns a process,
handshakes, opens a session, then tears down. Main turns pay the same process tax on every
message even when the conversation session id is reused. That overhead is pure waste once
a chat is active, and it dominates the cheap-model oneshot path users hit after every
turn.

Separately, an empty chat has no assistant message to feed a oneshot. Today the lifecycle
heuristic (`ds-explore`, `ds-spec`, …) is only a soft hint and never arms empty Enter. The
heuristic is the best available signal before a session starts — whatever it outputs
should be the initial suggestion, not explore-only special casing.

## Scope

```
caps/
├── harness/
│   ├── warm-runtime/     ← NEW
│   │   ├── spec.md
│   │   └── doc.md
│   └── grok/             (modified — process reuse; oneshot rotate)
│       ├── spec.md
│       └── doc.md
└── chat/
    └── default-prompts/  (modified — pre-oneshot + failed-oneshot heuristic)
        ├── spec.md
        └── doc.md
```

### New capabilities

- `harness/warm-runtime` — Per-chat worker owns a **main** runtime and a single
  **oneshot** runtime. Main and oneshot become process-hot lazily on the first user send.
  Title summaries and reply suggestions share the oneshot runtime and serialize on it.
  After each oneshot use, rotate to a fresh logical session off the hot path (N=1) so
  context does not accumulate. Cancel kills the main child; the next send re-warms.
  Harnesses that cannot reuse a process implement no-op hot (Claude stays cold in this
  change). No idle teardown.

### Modified capabilities

- `harness/grok` — Stop spawning a new `grok agent stdio` per main turn, title summary, or
  reply suggestion. Keep the child across calls; main continues to resume the conversation
  session id; oneshots do not accumulate prior oneshot context.

- `chat/default-prompts` — When no oneshot has armed a non-empty list yet, the effective
  empty-input list is the lifecycle heuristic alone (any stage the ladder produces, not
  only explore). A settled oneshot with parsed replies still replaces the list (oneshot
  only; heuristic not merged in). A failed or empty oneshot falls back to the heuristic
  when present. Pre-oneshot suggestions are ready without a model call.

### Out of scope

- Soft cancel that keeps the main process alive across interrupt
- Idle eviction of warm children
- Separate oneshot processes for title vs reply
- Making the Claude harness process-hot
- App-global or cross-chat process pools (must stay per-chat handle)
- Changing how the lifecycle heuristic is computed (phase ladder stays as-is)

## Impact

```
duckboard                         duckchat
─────────                         ────────
AgentHandle  ── cmds ──►  worker
                            ├─ main runtime    (process-hot, chat session)
                            └─ oneshot runtime (process-hot, N=1 rotate)
title / reply  ── via handle ──┘

empty-input defaults (local):
  pre-oneshot / failed oneshot  →  heuristic list (no runtime)
  oneshot with replies          →  parse only
```

- **duckchat:** runtime contract + worker ownership; title and reply routed through handle
  commands instead of ad-hoc provider construction

- **duckboard:** first send triggers ensure-hot; oneshot dispatch via handle; effective
  default list seeds from and falls back to `obvious_command`

- **Grok:** real process reuse; **Claude:** cold-compatible no-op behind the same contract

- No user-facing API break beyond empty-input defaults becoming useful on new sessions
