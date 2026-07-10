# Chat obvious bubble

Lifecycle next-command as a greyed faux user bubble and ⌘↩ send path — independent of
oneshot composer suggestions.

## Surfaces

Two empty-composer affordances stay separate:

```
lifecycle next command (disk / phase ladder)
        │
        ├── obvious bubble + ⌘↩ / click  ──▶ send that command
        │
        └── soft hint only ──▶ oneshot ──▶ composer default-prompt list
                                              (empty Enter / Tab)
```

The bubble never shows oneshot `REPLY:` lines. The composer list never shows the lifecycle
command unless the oneshot happens to emit the same text.

## Visibility

```
| Condition                         | Bubble   |
|-----------------------------------|----------|
| Main turn streaming               | Hidden   |
| Composer non-empty                | Hidden   |
| No lifecycle next command         | Hidden   |
| Idle, empty composer, command set | Shown    |
| Oneshot pending (idle + command)  | Shown    |
```

While the oneshot is still in flight, the bubble remains available so the lifecycle path
does not wait on the model.

## Send text

The stored lifecycle command is shown and sent in empty-send form: bare names such as
`ds-explore` become `/ds-explore`. Values that already include a leading `/` are left
unchanged.

## Activation

⌘↩ (when the bubble is visible) or activating the bubble sends the send text through the
same path as typing that text and submitting. After send, the message is a normal user
bubble in history. Before activation, the ghost is view chrome only — not part of the
persisted transcript.

## When there is no command

Scopes without a lifecycle next command (for example capability-tree or codex sessions, or
an archived change) show no bubble and do not bind the lifecycle ⌘↩ path.
