# @ Chat obvious bubble

## ~ Display and activation

Each chip label places the hotkey before the action (e.g. `⌘1  /ds-step`, `⌘↩  Confirm`,
`⌘⌫  Reject`). Activation (matching key or chip click) sends the action string only
through the same path as typing that text and submitting.

```
| Role                         | Appearance (quiet tint) | Label form              | Send text              |
|------------------------------|-------------------------|-------------------------|------------------------|
| Numbered lifecycle (multi)   | light blue              | `⌘n  /ds-…`             | that `/ds-…`           |
| Enter dual (multi, no affirm)| green                   | `⌘↩  Apply` (friendly)  | first lifecycle `/ds-…`|
| Single lifecycle only        | green                   | `⌘1  /ds-…`             | that `/ds-…`           |
| Affirm                       | green                   | `⌘↩  Confirm` (etc.)    | Confirm / Commit / …   |
| Decline                      | red                     | `⌘⌫  Reject`            | Reject                 |
```

When there are two or more lifecycle options and no affirm, the first lifecycle option is
dual-presented: a blue numbered chip in order, plus a green enter chip at the bottom of
the chrome with a friendly name (strip `/ds-` / `ds-`, title-case — e.g. `/ds-apply` →
`Apply`). Both send the original `/ds-…` string. A single lifecycle option (e.g.
`/ds-explore`) or affirm-only chrome (Commit, Create change) stays one green chip — no
dual row. When affirm is present, lifecycle chips are numbered only; green is the affirm
chip.

Chips sit in the chat scroll column after transcript content (not an overlay). When the
natural height of messages plus chrome is shorter than the chat viewport, a top pad above
the chrome pins the chips to the bottom of the history pane above the composer. When
content already fills or exceeds the viewport, the pad is zero and chips follow the last
message in document order.

Before activation, chips are view chrome only — not part of the persisted transcript.
After send, the message is a normal user bubble in history.
