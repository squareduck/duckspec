# @ Chat obvious bubble

## ~ Categories and keys

```
| Kind      | Content                           | Key   | Send text                        |
|-----------|-----------------------------------|-------|----------------------------------|
| Lifecycle | ordered `/ds-*`                   | ⌘1…⌘n | that `/ds-*`                     |
| Affirm    | Confirm, Commit, or Create change | ⌘↩    | Confirm / Commit / Create change |
| Decline   | Reject                            | ⌘⌫    | Reject                           |
```

⌘↩ resolves to affirm when present, otherwise the first lifecycle option, otherwise
nothing. ⌘⌫ sends `Reject` only when decline is present.

## ~ Composition

```
| Phase / condition                              | Lifecycle                              | Affirm        | Decline |
|------------------------------------------------|----------------------------------------|---------------|---------|
| Exploration, empty session                     | `/ds-explore`                          | —             | —       |
| Exploration, nonempty                          | —                                      | Create change | —       |
| Empty change, no reviews                       | `/ds-propose`                          | —*            | —*      |
| Proposal, no design, no caps, no reviews       | `/ds-design`, `/ds-spec`               | *             | *       |
| Design, no caps, no reviews                    | `/ds-spec`, `/ds-step`                 | *             | *       |
| Caps, no steps, no reviews                     | `/ds-step`, `/ds-archive`              | *             | *       |
| Open steps, no reviews                         | `/ds-apply`, `/ds-review`              | —             | —       |
| Open steps + review                            | `/ds-apply`                            | Confirm       | Reject  |
| No open steps + review                         | `/ds-step`, `/ds-spec`, `/ds-archive`  | Confirm       | Reject  |
| All steps done, no reviews                     | `/ds-archive`, `/ds-review`            | —             | —       |
| Archived + nonempty + dirty VCS                | —                                      | Commit        | —       |
| Caps / Codex scopes                            | —                                      | —             | —       |
```

`*` Confirm + Reject when the change session is non-empty and the change has no steps (and
no review is required for that pre-step case). Any nonempty session with at least one
review also gets Confirm + Reject, so write-approval during post-review `/ds-step` /
`/ds-spec` has a green ⌘↩ path. Open steps without a review stay lifecycle-only. Create
change and Commit are affirm-only (no Reject). When affirm is present, ⌘↩ sends Confirm
rather than the first lifecycle option.
