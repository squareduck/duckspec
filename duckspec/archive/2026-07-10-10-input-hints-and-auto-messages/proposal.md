# Input hints and auto messages

Global toggles for agent under-input suggestions (default off) and obvious chip chrome
(default on), plus restoring a disk-seeded under-input list on truly empty sessions so
empty Enter works without waiting on a model.

## Motivation

After splitting obvious chrome from oneshot defaults, empty sessions (new chat on an
ongoing change, fresh exploration) have no under-input path — oneshot needs a prior
assistant turn. Users also need app-wide control: agent suggestions can be noisy or
costly, and some users want chips gone entirely. Naming should match the two surfaces:
**input hints** vs **auto messages**.

## Scope

```
caps/chat/
├── default-prompts/   (modified — input-hints surface)
└── obvious-bubble/    (modified — auto-messages gate)
```

### New capabilities

None.

### Modified capabilities

- `chat/default-prompts` — Empty session (`messages` empty) with a first lifecycle option
  yields an under-input list of that single formatted command (ready immediately; Enter
  sends; same chrome as the agent list). Non-empty session: list is oneshot-only when
  agent hints are enabled; disk never merges with agent results. Agent oneshot launch is
  gated by a global setting (default **off**). Soft oneshot heuristic is unchanged when
  the agent path runs.

- `chat/obvious-bubble` — Global **auto messages** setting (default **on**). When off, the
  entire chip system is dark: no lifecycle / affirm / decline chrome and no ⌘ resolution
  from chrome.

### Also (no new capability)

- Settings UI and `config.toml` keys for both toggles

- Docs and summaries reframe “default prompts / obvious bubble” as input hints / auto
  messages without renaming capability directory paths

### Out of scope

- Ghost text inside the `TextEdit` field
- A separate toggle for empty-session disk seed (always on when applicable)
- Multi-option disk list under the input (seed is first lifecycle only)
- Per-project or per-session overrides
- Renaming capability directories
- Changing lifecycle ladder composition
- Merging agent list with disk seed when the session is non-empty

## Impact

```
empty session ──▶ first lifecycle ──▶ under-input list ──▶ Enter
                     │
                     └── auto messages ON ──▶ chips (parallel)

non-empty + agent ON  ──▶ oneshot ──▶ under-input list
non-empty + agent OFF ──▶ no under-input list
auto messages OFF     ──▶ no chips / no chrome hotkeys
```

- duckboard: `Config` + Settings checkboxes; gate oneshot launch and chrome visibility;
  re-seed empty-session effective list from lifecycle\[0]

- Spec deltas for the two chat caps (today’s “heuristic never seeds list” rule is wrong
  for empty sessions)
