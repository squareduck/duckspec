# @ Chat obvious bubble

Ranked lifecycle `/ds-*` action chips plus optional affirm and decline, with key-first
labels and dual-purpose ⌘↩ — independent of oneshot composer suggestions.

## - Send text

## - Activation

## - When there is no command

## ~ Surfaces

Two empty-composer affordances stay separate:

```
disk phase + session empty? + VCS dirty
        │
        ├── obvious chrome chips + keys / click  ──▶ send action text
        │
        └── soft hint (first lifecycle only) ──▶ oneshot ──▶ composer list
                                                      (empty Enter / Tab)
```

Chrome never shows oneshot `REPLY:` lines. The composer list never shows lifecycle options
unless the oneshot happens to emit the same text.

Chips are action affordances (hotkey then action), not faux user messages.

## ~ Visibility

```
| Condition                              | Chrome   |
|----------------------------------------|----------|
| Main turn streaming                    | Hidden   |
| Composer non-empty                     | Hidden   |
| Empty chrome                           | Hidden   |
| Idle, empty composer, non-empty chrome | Shown    |
| Oneshot pending (idle + chrome)        | Shown    |
```

While the oneshot is still in flight, chrome remains available so the lifecycle path does
not wait on the model.

## + Categories and keys

```
| Kind      | Content              | Key   | Send text        |
|-----------|----------------------|-------|------------------|
| Lifecycle | ordered `/ds-*`      | ⌘1…⌘n | that `/ds-*`     |
| Affirm    | Confirm or Commit    | ⌘↩    | Confirm / Commit |
| Decline   | Reject               | ⌘⌫    | Reject           |
```

⌘↩ resolves to affirm when present, otherwise the first lifecycle option, otherwise
nothing. ⌘⌫ sends `Reject` only when decline is present.

## + Composition

```
| Phase / condition                         | Lifecycle                         | Affirm  | Decline |
|-------------------------------------------|-----------------------------------|---------|---------|
| Exploration, empty session                | `/ds-explore`                     | —       | —       |
| Exploration, nonempty                     | —                                 | —       | —       |
| Empty change                              | `/ds-propose`                     | —*      | —*      |
| Proposal, no design, no caps              | `/ds-design`, `/ds-spec`          | *       | *       |
| Design, no caps                           | `/ds-spec`                        | *       | *       |
| Caps, no steps                            | `/ds-step`, `/ds-spec`, `/ds-archive` | *   | *       |
| Open steps                                | `/ds-apply`, `/ds-review`         | *       | *       |
| All steps done                            | `/ds-archive`, `/ds-review`       | *       | *       |
| Archived + nonempty + dirty VCS           | —                                 | Commit  | —       |
| Caps / Codex scopes                       | —                                 | —       | —       |
```

`*` Gate row (Confirm + Reject) only when the change session transcript is non-empty.
Empty change sessions show lifecycle only.

## + Display and activation

Each chip label places the hotkey before the action (e.g. `⌘1  /ds-step`, `⌘↩  Confirm`,
`⌘⌫  Reject`). Activation (matching key or chip click) sends the action string only
through the same path as typing that text and submitting.

Before activation, chips are view chrome only — not part of the persisted transcript.
After send, the message is a normal user bubble in history.

## + Soft hint

The first lifecycle option (when any) remains a soft hint on the reply-suggestion oneshot
request. Orientation's single suggested next stage matches that same first lifecycle
option. The oneshot composer list is otherwise independent.
