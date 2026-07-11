# @ Chat default prompts

## ~ Surfaces

Two empty-composer surfaces stay separate:

```
| Surface      | Source                                                         | Empty-input chrome                         | Send key  |
|--------------|----------------------------------------------------------------|--------------------------------------------|-----------|
| Next actions | Empty session: scope lifecycle[0]; else trailing `next`        | Ghost + optional tab marker before ghost   | Enter     |
| Oneshot hint | Cheap-model `REPLY:` when agent input hints on                 | Single under-input row + ⌘↩ marker         | Cmd-Enter |
```

Empty-session lifecycle[0] is scope-derived: exploration → explore stage command; change →
first option of that change's artifact/step ladder; caps and codex → none. That bootstrap
does not use the agent input hints setting. Next actions never list under the input.
Oneshot never fills the next-action list or ghost. Missing trailing `next` after the first
turn means no next-action ghost — not a disk-phase fallback.

## ~ Next-action list

```
session empty + lifecycle[0] (scope-derived)
    │
    ▼
next-actions = [lifecycle[0]]   (ready immediately; not gated by agent input hints)

session empty, no lifecycle[0]
    │
    ▼
next-actions = []

session non-empty
    │
    ▼
trailing next actions from last assistant   (0–3 send tokens)
    │
    └── no trailing next ──▶ next-actions = []
```

Empty Enter sends the active send token. Tab / Shift-Tab cycle when there are two or more
actions; a small tab-available marker appears **before** the ghost only then. Ghost text
is the active send token via the empty-input placeholder (hidden while streaming).
