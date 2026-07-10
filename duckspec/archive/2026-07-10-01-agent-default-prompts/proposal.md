# Agent default prompts

After each agent turn, duckboard asks the session harness’s cheapest model for 1–3 likely
user replies (skill and stage calls preferred), shows them as empty-input defaults with
Tab cycling, and always appends the lifecycle heuristic as the last unique fallback.

## Motivation

Empty Enter today only offers a disk-derived `/ds-*` stage from `change_scope_facts`. That
ladder is correct for orientation, but blind to conversation-local steers: skip design and
go `/ds-spec`, post-`/ds-review` forks between stages, or a plain “yes, continue” when the
agent asks. Those moments are when a good default matters most — and when the heuristic is
most often wrong or useless. The agent’s last message already carries the real next
action; we should turn that signal into empty-input defaults without replacing the ladder
as a stable fallback.

## Scope

```
caps/
└── chat/
    ├── composer-footer/     (unchanged — resend / usage / model chrome)
    ├── persistence/
    ├── transcript/
    └── default-prompts/     ← NEW
```

### New capabilities

- `chat/default-prompts` — Conversation-local empty-input defaults:

  1. **Oneshot suggestions** — after a turn completes, call the session harness’s cheapest
     model (sibling of title summary); prime for skill/stage calls when the agent is
     steering workflow; return 0–3 replies in a strict parseable form; do not feed the
     heuristic into the prompt

  2. **Merge and dedupe** — agent replies first (model order), lifecycle `obvious_command`
     always last when present and unique; slash forms normalize for equality (`/ds-spec` ≡
     `ds-spec`); unknown `/…` from the model is kept

  3. **Empty-input UI** — show the full list when the composer is empty; Tab / Shift-Tab
     cycle the active option; Enter sends the active option; when suggestions are not
     ready or fail, the list is heuristic-only (same as today)

### Modified capabilities

- None. `session/scope` orientation and `change_scope_facts` stay disk-only; this layer
  sits on top of the existing heuristic rather than changing it.

### Out of scope

- Changing the lifecycle ladder or teaching it about reviews
- Feeding the heuristic into the oneshot prompt
- Auto-send without Enter
- Persisting suggestions across restarts (ephemeral per turn is enough)
- Composer footer chrome (`chat/composer-footer`)

## Impact

```
TurnComplete ──► duckchat oneshot (cheap model) ──► parse reply lines
                         │
                         ▼
              agent replies ⊕ heuristic (last, deduped)
                         │
                         ▼
              empty composer: list + Tab cycle + Enter
```

- **duckchat:** new `Provider` oneshot sibling of `title_summary` for both claude-code and
  grok (same cheap-model pick pattern)

- **duckboard:** replace single-option empty-Enter / placeholder path with a multi-option
  prompt list and keyboard cycling

- No duckpond or `duckspec/` artifact schema changes
