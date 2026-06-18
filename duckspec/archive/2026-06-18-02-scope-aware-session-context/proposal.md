# Session scope orientation

Make the per-session orientation blurb duckboard sends a coding agent reliable,
self-describing, and authoritative — so the agent knows which change it is scoped to, and
that change's phase and progress, without re-asking or re-deriving from `ds status`.

## Motivation

When several changes are live, asking the agent to `/ds-archive` (or `/ds-apply`) makes it
ask "which one?" — even though the session is already bound to a specific change. Two
causes compound:

1. The scope blurb rides `--append-system-prompt`, which the Claude Code CLI silently
   drops. AGENTS.md was already moved off this channel for the same reason; the scope line
   was left behind and confirmed missing in practice.

2. The archive and apply templates instruct the agent to run `ds status` to *identify* the
   change — ambiguous when multiple changes are active.

The blurb is also thin: it carries only the change name, with no phase or progress, so
even when it arrives the agent learns little.

## Scope

```
caps/
├── archive/ audit/ ideas/ merge/ parse/   (existing — library)
└── session/                                ← NEW area
    └── scope/
        └── spec.md                         ← per-session scope orientation
```

### New capabilities

- `session/scope` — the per-session scope orientation blurb: delivered on a reliable
  channel (the first-turn message body, the same path AGENTS.md already uses), with
  scope-kind-specific contents. For a change, enriched with phase, aggregate step/task
  progress, and the suggested next stage, read from data duckboard already loads
  (`has_proposal`, `has_design`, step completion).

### Modified capabilities

- None in `caps/`. The agent-command templates
  (`crates/duckspec/content/templates/archive.md`, `apply.md`, and siblings that say "run
  `ds status` to identify the change") are prose content, not duckspec capabilities. They
  are reworded to default to the scoped change and treat `ds status` as a disambiguation
  fallback only.

### Out of scope

- Any `ds` CLI behavior — no `--scope` flag, no `ds archive` auto-resolution of the
  current change.

- AGENTS.md priming, selection attachments, and idea-description injection stay untouched.

- The legacy no-session-id fallback path in `send_prompt_text` keeps its current behavior.

## Impact

```
duckboard/src/scope.rs            duckboard/src/area/interaction.rs
  CurrentScopeHook + SessionScope    send_prompt_text
  ↑ carry change state (phase,       ↑ blurb moves onto the priming
    progress); render enriched         message body instead of
    Change blurb                       --append-system-prompt
```

- `crates/duckboard/src/scope.rs` — enrich `CurrentScopeHook` and `SessionScope` to carry
  change state and render the enriched Change blurb.

- `crates/duckboard/src/area/interaction.rs` — deliver the blurb on the reliable message
  channel.

- `crates/duckspec/content/templates/` — scope-aware rewording of the archive and apply
  templates.

No `duckpond` (library) or CLI-logic changes. No breaking changes.
