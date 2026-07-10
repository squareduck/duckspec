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
affirm `Confirm` and decline `Reject` when either the change has at least one review, or
the change has no steps on disk — except on the Commit-only path below. When the session
is empty, the gate row SHALL be omitted. When the session is non-empty, the change has
steps on disk, and the change has no reviews, the gate row SHALL be omitted (lifecycle
chips only).

When the scope is an exploration and the session is non-empty, the chrome SHALL be affirm
`Create change` only — no lifecycle options and no `Reject`. The affirm send text SHALL be
the literal string `Create change`.

When the scope is an archived change, the session is non-empty, and the repository is
dirty, the chrome SHALL be affirm `Commit` only — no lifecycle options and no `Reject`.
Other archived cases SHALL yield empty chrome.

Caps and codex scopes SHALL yield empty chrome.

> test: code

### - Scenario: Caps without steps yield step then spec then archive

### ~ Scenario: Nonempty change session includes Confirm and Reject

- **GIVEN** an active change with a non-empty session transcript
- **AND** the change has no steps on disk
- **AND** the change has no reviews
- **AND** the change is not on the archived Commit-only path
- **WHEN** obvious chrome is composed
- **THEN** affirm is Confirm
- **AND** decline is present

> test: code

### + Scenario: Nonempty exploration yields Create change only

- **GIVEN** an exploration scope with a non-empty session transcript
- **WHEN** obvious chrome is composed
- **THEN** affirm is Create change
- **AND** there are no lifecycle options
- **AND** decline is absent

> test: code

### + Scenario: Design without caps yields spec then step

- **GIVEN** an active change with a design, no capabilities, and no reviews
- **WHEN** obvious chrome is composed
- **THEN** the lifecycle options are `/ds-spec` and `/ds-step` in that order

> test: code

### + Scenario: Caps without steps yield step then archive

- **GIVEN** an active change with at least one capability, no steps, and no reviews
- **WHEN** obvious chrome is composed
- **THEN** the lifecycle options are `/ds-step` and `/ds-archive` in that order

> test: code

### + Scenario: Open steps yield apply then review without gate

- **GIVEN** an active change with at least one incomplete step and no reviews
- **AND** a non-empty session transcript
- **WHEN** obvious chrome is composed
- **THEN** the lifecycle options are `/ds-apply` and `/ds-review` in that order
- **AND** affirm is absent
- **AND** decline is absent

> test: code

### + Scenario: Open steps with review yield apply only with gate

- **GIVEN** an active change with at least one incomplete step and at least one review
- **AND** a non-empty session transcript
- **WHEN** obvious chrome is composed
- **THEN** the only lifecycle option is `/ds-apply`
- **AND** affirm is Confirm
- **AND** decline is present

> test: code

### + Scenario: No open steps with review yield step then spec then archive with gate

- **GIVEN** an active change with no incomplete steps and at least one review

- **AND** a non-empty session transcript

- **WHEN** obvious chrome is composed

- **THEN** the lifecycle options are `/ds-step`, `/ds-spec`, and `/ds-archive` in that
  order

- **AND** affirm is Confirm

- **AND** decline is present

> test: code

## @ Requirement: Key resolution

When the chrome is visible, key activation SHALL resolve to a send string as follows: ⌘↩
yields the affirm send text when affirm is present, otherwise the first lifecycle option
when any exist, otherwise no send; ⌘⌫ yields `Reject` when decline is present, otherwise
no send; ⌘*n* for digit *n* from 1 through 9 yields the *n*th lifecycle option when that
index exists, otherwise no send. When the chrome is not visible, every such activation
SHALL be a no-op (no send).

The resolved send text SHALL be the action string only (lifecycle empty-send form,
`Confirm`, `Commit`, `Create change`, or `Reject`). It SHALL NOT be taken from the oneshot
default-prompt list, even when that list is non-empty and differs.

> test: code

### ~ Scenario: Cmd-Enter sends affirm when present

- **GIVEN** visible chrome with affirm Confirm, Commit, or Create change
- **AND** one or more lifecycle options
- **WHEN** ⌘↩ activation is resolved
- **THEN** the send text is the affirm action string
- **AND** the send text is not a lifecycle option

> test: code

## @ Requirement: Chip display

### - Scenario: Affirm chip label is hotkey then Confirm or Commit

### + Scenario: Affirm chip label is hotkey then Confirm, Commit, or Create change

- **GIVEN** affirm Create change
- **WHEN** the chip label is derived
- **THEN** the label starts with the ⌘↩ hotkey
- **AND** the label includes `Create change`
- **AND** the send text is exactly `Create change`

> test: code
