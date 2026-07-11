# Chat fast response

Source-neutral option chips for mid-turn structured choices: ordered options with ⌘-number
activation, ephemeral view layout, and empty-send formatting for bare skill names.
Ordinary refresh leaves the shell empty; a live user-choice request may fill it. Freeform
submit while awaiting is a custom answer to the pending question. Esc dismisses without
answering.

## Shell model

Fast response is a thin option shell, not a lifecycle ladder:

```
| Field   | Role                                          |
|---------|-----------------------------------------------|
| options | Ordered choices; ⌘1…⌘n when chips are visible |
```

There is no cancel chip and no ⌘⌫ binding on the shell. Turn cancel (esc esc) completes a
parked choice as cancelled on the agent wire. Composer submit while awaiting completes it
as a custom freeform answer.

Chips are view chrome only until activation. Option activation and custom freeform both
finish the pending request in-band and do not invent a user transcript message for that
completion. Empty-send formatting (bare `ds-foo` → `/ds-foo`) remains available for other
empty-composer bootstrap consumers; it does not imply the shell is filled from disk phase.

## Visibility and keys

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
non-empty composer hides chips so typed text is not competing with option chrome (e.g.
future oneshot-hint fill). Oneshot pending under the input does not hide chips when the
other gates pass.

```
| Kind   | Key   | Result      |
|--------|-------|-------------|
| Option | ⌘1…⌘n | that option |
```

Chip labels put the hotkey before the action text; activation uses the option payload
only.

## Freeform while awaiting (custom answer)

When chips reflect a live user choice and the user types freeform text then submits:

```
awaiting choice + non-empty submit
        │
        ├─ complete pending choice as custom answer (freeform text)
        ├─ clear option shell
        └─ harness maps freeform into the question answer value
           (not cancel/skip + next user turn; not interrupt-queue only)
```

```
| Input            | Meaning        | Choice completion      |
|------------------|----------------|------------------------|
| ⌘n chip          | structured pick| selected option        |
| Composer Enter   | custom answer  | freeform text as answer|
| Esc esc          | dismiss        | cancelled              |
```

## Awaiting composer chrome

While awaiting a user choice, the whole composer section (input, footer strip, model
selector) uses the same quiet accent tint as numbered option chips so the strip reads as
the custom-answer surface and the model control does not stand out untinted. Tint clears
when the session is no longer awaiting.

## Population

Ordinary chat scopes leave options empty after refresh when the session is not awaiting a
user choice. A live structured question (or another later path) fills the shell without
rebuilding visibility, keys, chips, or bottom-pad layout. Refresh must not clear options
while a choice is pending.

## Layout

When chips are visible, they sit in the chat scroll column after transcript content. A top
pad pins short history so chips sit at the bottom of the viewport; tall content gets zero
pad and chips follow the last message.
