# @ Chat fast response

Source-neutral option chips for mid-turn structured choices and settled oneshot reply
suggestions: ordered options with ⌘-number activation, ephemeral view layout, and
empty-send formatting for bare skill names. A live user-choice request fills the shell for
in-band answers; settled oneshot hints may fill it for ordinary user-message sends when
eligible. Freeform submit while awaiting is a custom answer to the pending question.

## ~ Shell model

Fast response is a thin option shell with an activation source:

```
| Field   | Role                                                          |
|---------|---------------------------------------------------------------|
| options | Ordered choices; ⌘1…⌘n when chips are visible                 |
| source  | Why the shell is filled — drives activation                   |
```

```
| Source        | Filled by                         | Activation                         |
|---------------|-----------------------------------|------------------------------------|
| User choice   | Mid-turn structured question      | In-band answer (no new user msg)   |
| Oneshot hints | Settled freeform reply suggestions| Normal user message send           |
| (empty)       | Nothing                           | No-op                              |
```

There is no cancel chip and no ⌘⌫ binding on the shell. Turn cancel (esc esc) completes a
parked choice as cancelled on the agent wire. Composer submit while awaiting completes it
as a custom freeform answer.

Chips are view chrome only until activation. User-choice option activation and custom
freeform both finish the pending request in-band and do not invent a user transcript
message for that completion. Oneshot-hint activation sends the option text as a normal
user turn. Empty-send formatting (bare `ds-foo` → `/ds-foo`) remains available for other
empty-composer bootstrap consumers; it does not imply the shell is filled from disk phase.

## ~ Visibility and keys

```
| Condition                                               | Chips  |
|---------------------------------------------------------|--------|
| No options                                              | Hidden |
| Main turn open, not awaiting user                       | Hidden |
| Not awaiting, non-empty composer                        | Hidden |
| Idle, empty composer, non-empty options                 | Shown  |
| Awaiting user, empty composer, non-empty options        | Shown  |
| Awaiting user, non-empty composer (custom answer)       | Shown  |
```

While awaiting, chips stay visible as the user types a custom answer. When not awaiting, a
non-empty composer hides chips so typed text is not competing with option chrome
(including oneshot-hint fill).

```
| Kind   | Key   | Result      |
|--------|-------|-------------|
| Option | ⌘1…⌘n | that option |
```

Chip labels put the hotkey before the action text; activation uses the option payload
only.

## ~ Population

```
refresh / sync
    │
    ├─ awaiting user choice?  ──▶ keep user-choice fill
    ├─ oneshot eligible?      ──▶ options = settled REPLY list (oneshot source)
    └─ else                   ──▶ empty shell
```

A live structured question always overwrites oneshot fill. A late oneshot settle does not
replace a parked user choice. Ordinary scopes do not compose lifecycle phase chips into
the shell.
