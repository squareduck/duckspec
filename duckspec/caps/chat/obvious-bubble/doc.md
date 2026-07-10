# Chat obvious bubble

Lifecycle next-command as a greyed faux user bubble and ⌘↩ send path — independent of
oneshot composer suggestions.

**Auto messages:** ranked lifecycle `/ds-*` action chips plus optional affirm and decline,
with key-first labels and dual-purpose ⌘↩ — independent of under-input input hints, and
shown only when the global auto messages setting is enabled (default on).

## Surfaces

Two empty-composer affordances stay separate:

```
disk phase + session empty? + VCS dirty
        │
        ├── auto messages ON
        │     obvious chrome chips + keys / click  ──▶ send action text
        │     under-input input hints: always empty
        │
        └── auto messages OFF
              first lifecycle (empty session)
              or agent oneshot (non-empty, agent hints ON)
                    ──▶ under-input input hints (empty Enter / Tab)
```

Chrome never shows oneshot `REPLY:` lines. When auto messages is on, under-input input
hints are fully suppressed so chips alone provide lifecycle assistance. When auto messages
is off, chips and chrome hotkeys are dark; an empty session may still seed the under-input
list from the first lifecycle option so empty Enter works without chips.

Chips are action affordances (hotkey then action), not faux user messages.

## Visibility

```
| Condition                                       | Chrome   |
|-------------------------------------------------|----------|
| Auto messages disabled                          | Hidden   |
| Main turn streaming                             | Hidden   |
| Composer non-empty                              | Hidden   |
| Empty chrome                                    | Hidden   |
| Auto messages on, idle, empty composer, chrome  | Shown    |
| Oneshot pending (idle + chrome + auto on)       | Shown    |
```

While a reply-suggestion oneshot is still in flight, chrome remains available when auto
messages is on so the lifecycle path does not wait on the model.

## Categories and keys

```
| Kind      | Content                           | Key   | Send text                        |
|-----------|-----------------------------------|-------|----------------------------------|
| Lifecycle | ordered `/ds-*`                   | ⌘1…⌘n | that `/ds-*`                     |
| Affirm    | Confirm, Commit, or Create change | ⌘↩    | Confirm / Commit / Create change |
| Decline   | Reject                            | ⌘⌫    | Reject                           |
```

⌘↩ resolves to affirm when present, otherwise the first lifecycle option, otherwise
nothing. ⌘⌫ sends `Reject` only when decline is present.

## Composition

```
| Phase / condition                              | Lifecycle                                                    | Affirm        | Decline |
|------------------------------------------------|--------------------------------------------------------------|---------------|---------|
| Exploration, empty session                     | `/ds-explore`                                                | —             | —       |
| Exploration, nonempty                          | —                                                            | Create change | —       |
| Empty change, no reviews                       | `/ds-propose`                                                | —*            | —*      |
| Proposal, no design, no caps, no reviews       | `/ds-design`, `/ds-spec`                                     | *             | *       |
| Design, no caps, no reviews                    | `/ds-spec`, `/ds-step`                                       | *             | *       |
| Caps, no steps, no reviews                     | `/ds-step`, `/ds-archive`                                    | *             | *       |
| Open steps (with or without reviews)           | `/ds-apply`, `/ds-review`, `/ds-followup`                    | —‡            | —‡      |
| No open steps + review                         | `/ds-step`, `/ds-spec`, `/ds-review`, `/ds-followup`, `/ds-archive` | Confirm | Reject  |
| All steps done, no reviews                     | `/ds-archive`, `/ds-review`, `/ds-followup`                  | Confirm†      | Reject† |
| Archived + nonempty + dirty VCS                | —                                                            | Commit        | —       |
| Caps / Codex scopes                            | —                                                            | —             | —       |
```

`*` Confirm + Reject when the change session is non-empty and the change has no steps (and
no review is required for that pre-step case). Any nonempty session with at least one
review also gets Confirm + Reject, so write-approval during post-review rework and further
critique has a green ⌘↩ path.

`‡` Open steps without a review stay lifecycle-only. Open steps *with* a review keep the
same lifecycle list (`/ds-apply`, `/ds-review`, `/ds-followup`) and add Confirm + Reject
because a review is on file.

`†` Confirm + Reject when the session is non-empty — whenever lifecycle includes
`/ds-archive` (including all steps done with no reviews), so archive dry-run write
approval has the same gate without inspecting chat content. Empty sessions stay
lifecycle-only for that arm.

Create change and Commit are affirm-only (no Reject). When affirm is present, ⌘↩ sends
Confirm rather than the first lifecycle option.

Review and followup remain available whenever steps are open and whenever there are no
open steps (including after a first critique file exists), so re-review and re-followup
are always one chip away.

## Display and activation

Each chip label places the hotkey before the action (e.g. `⌘1  /ds-step`, `⌘↩  Confirm`,
`⌘⌫  Reject`). Activation (matching key or chip click) sends the action string only
through the same path as typing that text and submitting.

```
| Role                         | Appearance (quiet tint) | Label form              | Send text              |
|------------------------------|-------------------------|-------------------------|------------------------|
| Numbered lifecycle (multi)   | light blue              | `⌘n  /ds-…`             | that `/ds-…`           |
| Enter dual (multi, no affirm)| green                   | `⌘↩  Apply` (friendly)  | first lifecycle `/ds-…`|
| Single lifecycle only        | green                   | `⌘↩  Explore` (friendly)| that `/ds-…`           |
| Affirm                       | green                   | `⌘↩  Confirm` (etc.)    | Confirm / Commit / …   |
| Decline                      | red                     | `⌘⌫  Reject`            | Reject                 |
```

When there are two or more lifecycle options and no affirm, the first lifecycle option is
dual-presented: a blue numbered chip in order, plus a green enter chip at the bottom of
the chrome with a friendly name (strip `/ds-` / `ds-`, title-case — e.g. `/ds-apply` →
`Apply`). Both send the original `/ds-…` string. A single lifecycle option (e.g.
`/ds-explore`) is one green chip labeled `⌘↩  Explore` (friendly name, not numbered) — no
dual row. Affirm-only chrome (Commit, Create change) stays one green chip. When affirm is
present, lifecycle chips are numbered only; green is the affirm chip.

Chips sit inside the chat scroll column after transcript content (not an overlay, not
between the scroll viewport and the composer — so the input widget tree stays stable when
chrome shows or hides). When messages plus chrome are shorter than the chat viewport, a
top pad above the chrome pins chips to the bottom of the history pane; when content
already fills or exceeds the viewport, the pad is zero and chips follow the last message.

Before activation, chips are view chrome only — not part of the persisted transcript.
After send, the message is a normal user bubble in history.

## Soft hint

The first lifecycle option (when any) remains a soft hint on the reply-suggestion oneshot
request when agent input hints run and auto messages is off. Orientation's single suggested
next stage matches that same first lifecycle option. When auto messages is off and the
session is empty, that option may also seed the under-input input-hints list (separate
surface from chips).
