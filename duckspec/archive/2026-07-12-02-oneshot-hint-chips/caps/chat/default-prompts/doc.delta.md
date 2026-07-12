# @ Chat default prompts

Empty-composer next actions from lifecycle bootstrap or a trailing `next` meta card, shown
as ghost text with empty Enter and Tab cycle; optional settings-gated oneshot reply
suggestions (up to three freeform `REPLY:` lines) that may fill fast-response chips only
when there is no next-action ghost.

## - Oneshot presentation

## ~ Surfaces

Two empty-composer surfaces stay separate:

```
| Surface      | Source                                                         | Empty-input chrome                       | Activation              |
|--------------|----------------------------------------------------------------|------------------------------------------|-------------------------|
| Next actions | Empty session: scope lifecycle[0]; else trailing `next`        | Ghost + optional tab marker before ghost | Enter / Tab             |
| Oneshot hint | Cheap-model `REPLY:` when agent input hints on                 | Fast-response chips (0–3) when eligible  | ⌘n / click (chip send)  |
```

Empty-session lifecycle[0] is scope-derived: exploration → explore stage command; change →
first option of that change's artifact/step ladder; caps and codex → none. That bootstrap
does not use the agent input hints setting. Next actions never list under the input and
never fill chips. Oneshot never fills the next-action list or ghost. Missing trailing
`next` after the first turn means no next-action ghost — not a disk-phase fallback.

There is no under-input oneshot row, no oneshot loading strip, and no empty Cmd-Enter path
for oneshot suggestions. Empty Enter remains next-action only.

## ~ Oneshot pipeline

When agent input hints is enabled (default off), after a non-priming turn the harness may
run a short oneshot only when the next-action list is empty after that turn's refresh. The
request embeds the **full** last assistant message and preceding user message (no line
truncation, no lifecycle heuristic, no slash-command priming list). The instruction asks
for up to three plain `REPLY:` lines in order: most likely reply, alternative reply, and
negative or decline reply (omit a line when it does not fit) — freeform user text, not
stage-command autocomplete.

```
TurnComplete + agent hints ON + next-actions empty
    │
    ▼
cheap-model oneshot
    │
    ├── REPLY: lines (0–3)  ──▶ settled list → chip fill when eligible
    └── fail / timeout / empty  ──▶ ready empty list (no chips)
```

While oneshot is pending, no loading chrome is shown and chips are not filled from that
in-flight generation. Pending oneshot does **not** block empty Enter for next actions.
Eligibility for chip fill is evaluated separately (idle, not awaiting a user choice, empty
next-action list, non-empty settled list, hints on) — see chat fast response for shell
population and activation.
