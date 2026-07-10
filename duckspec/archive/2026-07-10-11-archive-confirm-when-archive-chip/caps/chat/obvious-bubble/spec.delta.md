# @ Chat obvious bubble

## @ Requirement: Chrome composition

Obvious chrome SHALL be composed only from disk lifecycle phase (including whether any
steps exist and whether any are incomplete), whether the change has at least one review,
whether the session transcript is empty, whether the change is archived, and whether the
repository working tree is dirty — never from oneshot default-prompt text.

Lifecycle options SHALL be ordered empty-send `/ds-*` strings by the first matching arm:

- empty exploration session: `/ds-explore` only
- nonempty exploration session: no lifecycle options
- open steps and at least one review: `/ds-apply` only
- open steps and no reviews: `/ds-apply`, then `/ds-review`
- no open steps and at least one review: `/ds-step`, then `/ds-spec`, then `/ds-archive`
- all steps complete and no reviews: `/ds-archive`, then `/ds-review`
- caps present, no steps, no reviews: `/ds-step`, then `/ds-archive`
- design present, no caps, no reviews: `/ds-spec`, then `/ds-step`
- proposal present, no design, no caps, no reviews: `/ds-design`, then `/ds-spec`
- empty change (no proposal), no reviews: `/ds-propose` only

When the scope is an active change and the session is non-empty, the chrome SHALL include
affirm `Confirm` and decline `Reject` when any of the following hold: the change has at
least one review; the change has no steps on disk; or the composed lifecycle options
include `/ds-archive` — except on the Commit-only path below. When the session is empty,
the gate row SHALL be omitted. When the session is non-empty, the change has steps on
disk, the change has no reviews, and the lifecycle options do not include `/ds-archive`,
the gate row SHALL be omitted (lifecycle chips only).

When the scope is an exploration and the session is non-empty, the chrome SHALL be affirm
`Create change` only — no lifecycle options and no `Reject`. The affirm send text SHALL be
the literal string `Create change`.

When the scope is an archived change, the session is non-empty, and the repository is
dirty, the chrome SHALL be affirm `Commit` only — no lifecycle options and no `Reject`.
Other archived cases SHALL yield empty chrome.

Caps and codex scopes SHALL yield empty chrome.

> test: code

### + Scenario: All steps complete nonempty session includes Confirm and Reject

- **GIVEN** an active change whose steps are all complete
- **AND** the change has no reviews
- **AND** a non-empty session transcript
- **WHEN** obvious chrome is composed
- **THEN** the lifecycle options are `/ds-archive` and `/ds-review` in that order
- **AND** affirm is Confirm
- **AND** decline is present

> test: code
