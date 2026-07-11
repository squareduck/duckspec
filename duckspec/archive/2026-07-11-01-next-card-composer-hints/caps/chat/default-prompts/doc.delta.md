# @ Chat default prompts

Empty-composer next actions from lifecycle bootstrap (empty session) or a trailing `next`
meta card (after the first turn), shown as ghost text with empty Enter and Tab cycle;
optional settings-gated oneshot reply suggestion as a single under-input line sent only
with empty Cmd-Enter.

## - Pipeline

## - Reply format and order

## - Readiness

## - Defaults list presentation

## + Surfaces

Two empty-composer surfaces stay separate:

```
| Surface        | Source                                      | Empty-input chrome                    | Send key   |
|----------------|---------------------------------------------|-------------------------------|------------|
| Next actions   | Empty session: lifecycle[0]; else trailing `next` | Ghost + optional tab marker before ghost | Enter      |
| Oneshot hint   | Cheap-model `REPLY:` when agent input hints on  | Single under-input row + ⌘↩ marker | Cmd-Enter  |
```

Next actions never list under the input. Oneshot never fills the next-action list or
ghost. Missing trailing `next` after the first turn means no next-action ghost — not a
disk-phase fallback.

## + Next-action list

```
session empty + lifecycle[0]
    │
    ▼
next-actions = [lifecycle[0]]   (ready immediately)

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

## + Oneshot pipeline

When agent input hints is enabled (default off), after a non-priming turn the harness may
run a short oneshot. The request embeds the **full** last assistant message and preceding
user message (no line truncation, no lifecycle heuristic, no slash-command priming list).
The instruction asks for a natural freeform user reply — not stage-command autocomplete —
as at most one `REPLY:` line.

```
TurnComplete + agent hints ON
    │
    ▼
cheap-model oneshot
    │
    ├── REPLY: line (0–1)  ──▶ under-input suggestion
    └── fail / timeout / empty  ──▶ no suggestion
```

While oneshot is pending, under-input shows loading (not the suggestion). Pending oneshot
does **not** block empty Enter for next actions. Empty Cmd-Enter sends the armed
suggestion only when ready; empty Enter and empty Shift-Enter never send it.

## + Oneshot presentation

The under-input row shows a legible Cmd-Enter marker before the suggestion text. Long
suggestions soft-wrap; full text stays visible without ellipsis.
