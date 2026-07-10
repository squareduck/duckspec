# @ Chat obvious bubble

## ~ Composition

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
