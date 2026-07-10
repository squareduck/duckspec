# Multi-option obvious chrome

Replace the single faux user lifecycle bubble with ranked action chips (lifecycle `/ds-*`,
affirm, decline), key-first labels, and dual-purpose ⌘↩ — so ambiguous stages offer a
short menu instead of one guessed command.

## Motivation

A single disk-derived next command is often wrong in the middle of multi-cap work (the
first written cap flips the ladder to `/ds-step`) and cannot express two different moves:
answer the agent at a write gate versus jump to another lifecycle stage. Session
sequencing (“Write the spec” after one turn) and parsing agent “Suggested next actions”
are heavier than showing a few ranked options with fixed hotkeys. ⌘↩ should mean the
obvious affirmative — Confirm or Commit when that row is shown, otherwise the best
`/ds-*`.

## Scope

```
caps/chat/
├── obvious-bubble/     (modified — multi-option chrome; not faux user bubbles)
└── default-prompts/    untouched this change

session/scope/          orientation keeps a single suggested next = lifecycle[0]
```

### New capabilities

- (none)

### Modified capabilities

- `chat/obvious-bubble` — Phase-derived ordered lifecycle `/ds-*` options (⌘1…⌘n);
  optional affirm (`Confirm` or `Commit` → ⌘↩); optional decline (`Reject` → ⌘⌫). Chips
  show **hotkey first**, then action text (e.g. `⌘1  /ds-step`). Visibility remains idle +
  empty composer + chrome non-empty. ⌘↩ sends affirm when present, else `lifecycle[0]`.
  Frozen phase table and gate rules from exploration (see below).

- `session/scope` — **no behavior change required** unless orientation text must be
  clarified; suggested next stage remains `lifecycle[0]` only.

### Out of scope

- Parsing agent `Suggested next actions:` (or other main-agent handoff scrape)
- Session FSM / alternating labels (“Write the spec”, “Write proposal”)
- Proposal expected-cap completeness for multi-cap “all done”
- Changing or removing oneshot `chat/default-prompts` (empty Enter / Tab path)
- New capability paths; questions-tool integration on the same chrome
- Freeform hotkeys beyond Confirm / Reject / Commit send strings

### Frozen chrome rules (contract for later stages)

**Categories and keys**

```
| Kind | Content | Key |
|------|---------|-----|
| Lifecycle | `/ds-*` only | ⌘1…⌘n |
| Affirm | `Confirm` or `Commit` | ⌘↩ when shown |
| Decline | `Reject` | ⌘⌫ when shown |
```

**⌘↩:** affirm if present, else `lifecycle[0]`, else no-op.

**Gate row (Confirm + Reject):** active change scope and non-empty session. Hidden for
exploration, caps, codex, empty change sessions, and the Commit-only post-archive case.

**Commit (affirm only):** archived change session, non-empty transcript, dirty VCS
(`changed_files` non-empty). No Reject beside Commit.

**Lifecycle ranks**

```
| Phase | Order (⌘1, ⌘2, …) |
|-------|-------------------|
| Caps, no steps | `/ds-step`, `/ds-spec`, `/ds-archive` |
| All steps done | `/ds-archive`, `/ds-review` |
| Open steps | `/ds-apply`, `/ds-review` |
| Design, no caps | `/ds-spec` |
| Proposal, no design | `/ds-design`, `/ds-spec` |
| Empty change | `/ds-propose` |
| Exploration, empty session | `/ds-explore` |
```

**Display:** chip is hotkey then action (`⌘1  /ds-step`); send text is the action string
only (`/ds-step`, `Confirm`, `Reject`, `Commit`).

## Impact

```
disk phase + session emptiness + VCS dirty
        → ObviousChrome { lifecycle[], affirm?, decline? }
        → chips [⌘n action] / [⌘↩ affirm] [⌘⌫ Reject]
        → send action text (not hotkey)
```

- duckboard: expand `obvious_bubble` pure API; replace single `obvious_command` with
  multi-option chrome; agent chat chips (not faux user bubbles); keybinds for ⌘↩ / ⌘⌫ /
  ⌘1…; refresh path uses phase table + archive/dirty/empty

- Spec rewrite or large delta on `chat/obvious-bubble` (single-bubble scenarios become
  multi-option / key resolution scenarios)

- Relies on existing archive scope migration and `changed_files` for Commit
