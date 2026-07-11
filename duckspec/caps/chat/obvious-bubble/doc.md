# Chat obvious bubble

Lifecycle next-command as a greyed faux user bubble and ⌘↩ send path — independent of
oneshot composer suggestions.

Generic empty-composer option chrome: ordered option chips with ⌘-number send, optional
cancel on ⌘⌫, ephemeral view layout, and empty-send formatting for bare skill names — not
populated by disk lifecycle or auto-messages in this capability's product path.

## Shell model

Obvious chrome is a thin option shell, not a lifecycle ladder:

```
| Field   | Role                                              |
|---------|---------------------------------------------------|
| options | Ordered send strings; ⌘1…⌘n when chrome is visible |
| cancel  | Optional send string on ⌘⌫                        |
```

Chips are view chrome only until activation sends a normal user message. Empty-send
formatting (bare `ds-foo` → `/ds-foo`) remains available for other empty-composer
bootstrap consumers; it does not imply chrome is filled from disk phase.

## Visibility and keys

```
| Condition                                      | Chrome |
|------------------------------------------------|--------|
| Main turn streaming                            | Hidden |
| Composer non-empty                             | Hidden |
| No options and no cancel                       | Hidden |
| Idle, empty composer, non-empty options/cancel | Shown  |
```

There is no auto-messages setting. Oneshot pending under the input does not hide chrome
when the other gates pass.

```
| Kind    | Key   | Send text        |
|---------|-------|------------------|
| Option  | ⌘1…⌘n | that option      |
| Cancel  | ⌘⌫    | cancel string    |
```

Chip labels put the hotkey before the action text; activation sends the action string
only.

## Population

Ordinary chat scopes leave options and cancel empty after refresh. Structured questions
(or another later path) can fill the shell without rebuilding visibility, keys, chips, or
bottom-pad layout.

## Layout

When chrome is visible, chips sit in the chat scroll column after transcript content. A
top pad pins short history so chips sit at the bottom of the viewport; tall content gets
zero pad and chips follow the last message.
