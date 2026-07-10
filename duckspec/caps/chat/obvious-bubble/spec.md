# Chat obvious bubble

Lifecycle next-command as a greyed faux user bubble and ⌘↩ send path — independent of
oneshot composer suggestions.

Auto messages: ranked lifecycle `/ds-*` action chips plus optional affirm and decline,
with key-first labels and dual-purpose ⌘↩ — independent of under-input input hints, and
shown only when the global auto messages setting is enabled (default on).

## Requirement: Ephemeral chrome

Obvious chrome chips SHALL NOT be stored in the session transcript until activation
produces a real user message. While only shown as view chrome, they SHALL NOT appear as
committed user messages in the session.

> test: code

### Scenario: Visible chrome is not a stored user message

- **GIVEN** obvious chrome is shown
- **AND** no chrome action has been activated
- **WHEN** the session transcript is inspected
- **THEN** it does not contain a user message whose sole purpose is the chrome chip

> test: code
> - crates/duckboard/src/obvious_bubble.rs:372

## Requirement: Lifecycle option formatting

When a lifecycle option is derived from a bare skill name, its send text SHALL be that
name in empty-send form with a single leading `/` (e.g. `ds-explore` becomes
`/ds-explore`). A lifecycle option that already begins with `/` SHALL be kept as stored.
Empty or blank skill names SHALL not produce a lifecycle send string.

> test: code

### Scenario: Bare skill name formats with leading slash

- **GIVEN** a lifecycle skill name stored without a leading slash
- **WHEN** the lifecycle send text is derived
- **THEN** the send text is that name with a single leading `/`

> test: code
> - crates/duckboard/src/obvious_bubble.rs:174

### Scenario: Already-slashed command is preserved

- **GIVEN** a lifecycle skill name that already begins with `/`
- **WHEN** the lifecycle send text is derived
- **THEN** the send text equals the stored name

> test: code
> - crates/duckboard/src/obvious_bubble.rs:190

## Requirement: Chrome composition

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

### Scenario: All steps complete yield archive then review

- **GIVEN** an active change whose steps are all complete
- **WHEN** obvious chrome is composed
- **THEN** the lifecycle options are `/ds-archive` and `/ds-review` in that order

> test: code
> - crates/duckboard/src/area/change.rs:2229

### Scenario: Nonempty change session includes Confirm and Reject

- **GIVEN** an active change with a non-empty session transcript
- **AND** the change has no steps on disk
- **AND** the change has no reviews
- **AND** the change is not on the archived Commit-only path
- **WHEN** obvious chrome is composed
- **THEN** affirm is Confirm
- **AND** decline is present

> test: code
> - crates/duckboard/src/area/change.rs:2263

### Scenario: Empty change session omits gate row

- **GIVEN** an active change with an empty session transcript
- **WHEN** obvious chrome is composed
- **THEN** affirm is absent
- **AND** decline is absent

> test: code
> - crates/duckboard/src/area/change.rs:2337

### Scenario: Archived dirty nonempty session yields Commit only

- **GIVEN** a session scoped to an archived change
- **AND** a non-empty session transcript
- **AND** a dirty repository working tree
- **WHEN** obvious chrome is composed
- **THEN** affirm is Commit
- **AND** there are no lifecycle options
- **AND** decline is absent

> test: code
> - crates/duckboard/src/area/change.rs:2347

### Scenario: Empty exploration yields explore only

- **GIVEN** an exploration scope with an empty session transcript
- **WHEN** obvious chrome is composed
- **THEN** the only lifecycle option is `/ds-explore`
- **AND** affirm is absent
- **AND** decline is absent

> test: code
> - crates/duckboard/src/area/change.rs:2362

### Scenario: Nonempty exploration yields Create change only

- **GIVEN** an exploration scope with a non-empty session transcript
- **WHEN** obvious chrome is composed
- **THEN** affirm is Create change
- **AND** there are no lifecycle options
- **AND** decline is absent

> test: code
> - crates/duckboard/src/area/change.rs:2211

### Scenario: Design without caps yields spec then step

- **GIVEN** an active change with a design, no capabilities, and no reviews
- **WHEN** obvious chrome is composed
- **THEN** the lifecycle options are `/ds-spec` and `/ds-step` in that order

> test: code
> - crates/duckboard/src/area/change.rs:2199

### Scenario: Caps without steps yield step then archive

- **GIVEN** an active change with at least one capability, no steps, and no reviews
- **WHEN** obvious chrome is composed
- **THEN** the lifecycle options are `/ds-step` and `/ds-archive` in that order

> test: code
> - crates/duckboard/src/area/change.rs:2187

### Scenario: Open steps yield apply then review without gate

- **GIVEN** an active change with at least one incomplete step and no reviews
- **AND** a non-empty session transcript
- **WHEN** obvious chrome is composed
- **THEN** the lifecycle options are `/ds-apply` and `/ds-review` in that order
- **AND** affirm is absent
- **AND** decline is absent

> test: code
> - crates/duckboard/src/area/change.rs:2277

### Scenario: Open steps with review yield apply only with gate

- **GIVEN** an active change with at least one incomplete step and at least one review
- **AND** a non-empty session transcript
- **WHEN** obvious chrome is composed
- **THEN** the only lifecycle option is `/ds-apply`
- **AND** affirm is Confirm
- **AND** decline is present

> test: code
> - crates/duckboard/src/area/change.rs:2293

### Scenario: No open steps with review yield step then spec then archive with gate

- **GIVEN** an active change with no incomplete steps and at least one review

- **AND** a non-empty session transcript

- **WHEN** obvious chrome is composed

- **THEN** the lifecycle options are `/ds-step`, `/ds-spec`, and `/ds-archive` in that
  order

- **AND** affirm is Confirm

- **AND** decline is present

> test: code
> - crates/duckboard/src/area/change.rs:2314

### Scenario: All steps complete nonempty session includes Confirm and Reject

- **GIVEN** an active change whose steps are all complete
- **AND** the change has no reviews
- **AND** a non-empty session transcript
- **WHEN** obvious chrome is composed
- **THEN** the lifecycle options are `/ds-archive` and `/ds-review` in that order
- **AND** affirm is Confirm
- **AND** decline is present

> test: code
> - crates/duckboard/src/area/change.rs:2243

## Requirement: Chrome visibility

Obvious chrome SHALL be shown only when all of the following hold: the global auto
messages setting is enabled, the main agent turn is not in progress, the composer input is
empty, and the chrome is non-empty (at least one lifecycle option, affirm, or decline).
The auto messages setting SHALL default to enabled. When auto messages is disabled, the
chrome SHALL NOT be shown even if the other gates would allow it. A pending or settled
oneshot for under-input input hints SHALL NOT hide the chrome when those gates hold. The
chrome SHALL NOT be shown when any gate fails.

> test: code

### Scenario: Idle empty composer with chrome shows chrome

- **GIVEN** auto messages enabled
- **AND** non-empty obvious chrome for the session
- **AND** an empty composer input
- **AND** no main agent turn in progress
- **WHEN** chrome visibility is evaluated
- **THEN** the chrome is shown

> test: code
> - crates/duckboard/src/obvious_bubble.rs:202

### Scenario: Streaming hides chrome

- **GIVEN** auto messages enabled
- **AND** non-empty obvious chrome for the session
- **AND** an empty composer input
- **AND** a main agent turn in progress
- **WHEN** chrome visibility is evaluated
- **THEN** the chrome is not shown

> test: code
> - crates/duckboard/src/obvious_bubble.rs:210

### Scenario: Non-empty composer hides chrome

- **GIVEN** auto messages enabled
- **AND** non-empty obvious chrome for the session
- **AND** a non-empty composer input
- **AND** no main agent turn in progress
- **WHEN** chrome visibility is evaluated
- **THEN** the chrome is not shown

> test: code
> - crates/duckboard/src/obvious_bubble.rs:217

### Scenario: Empty chrome is hidden

- **GIVEN** auto messages enabled
- **AND** empty obvious chrome for the session
- **AND** an empty composer input
- **AND** no main agent turn in progress
- **WHEN** chrome visibility is evaluated
- **THEN** the chrome is not shown

> test: code
> - crates/duckboard/src/obvious_bubble.rs:224

### Scenario: Oneshot pending does not hide chrome when otherwise visible

- **GIVEN** auto messages enabled
- **AND** non-empty obvious chrome for the session
- **AND** an empty composer input
- **AND** no main agent turn in progress
- **AND** a pending reply-suggestion oneshot
- **WHEN** chrome visibility is evaluated
- **THEN** the chrome is shown

> test: code
> - crates/duckboard/src/obvious_bubble.rs:232

### Scenario: Auto messages disabled hides chrome

- **GIVEN** auto messages disabled
- **AND** non-empty obvious chrome for the session
- **AND** an empty composer input
- **AND** no main agent turn in progress
- **WHEN** chrome visibility is evaluated
- **THEN** the chrome is not shown

> test: code
> - crates/duckboard/src/obvious_bubble.rs:240

### Scenario: Default auto messages setting is enabled

- **GIVEN** application config defaults
- **WHEN** the auto messages setting is read
- **THEN** it is enabled

> test: code
> - crates/duckboard/src/config.rs:256

## Requirement: Key resolution

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

### Scenario: Cmd-Enter sends affirm when present

- **GIVEN** visible chrome with affirm Confirm, Commit, or Create change
- **AND** one or more lifecycle options
- **WHEN** ⌘↩ activation is resolved
- **THEN** the send text is the affirm action string
- **AND** the send text is not a lifecycle option

> test: code
> - crates/duckboard/src/obvious_bubble.rs:252

### Scenario: Cmd-Enter sends first lifecycle when affirm absent

- **GIVEN** visible chrome with no affirm
- **AND** at least one lifecycle option
- **WHEN** ⌘↩ activation is resolved
- **THEN** the send text equals the first lifecycle option

> test: code
> - crates/duckboard/src/obvious_bubble.rs:262

### Scenario: Cmd-Backspace sends Reject when decline set

- **GIVEN** visible chrome with decline present
- **WHEN** ⌘⌫ activation is resolved
- **THEN** the send text is `Reject`

> test: code
> - crates/duckboard/src/obvious_bubble.rs:270

### Scenario: Cmd-digit sends matching lifecycle option

- **GIVEN** visible chrome with at least two lifecycle options
- **WHEN** ⌘2 activation is resolved
- **THEN** the send text equals the second lifecycle option

> test: code
> - crates/duckboard/src/obvious_bubble.rs:279

### Scenario: Resolution is a no-op when chrome not visible

- **GIVEN** chrome that is not visible
- **WHEN** ⌘↩, ⌘⌫, or ⌘1 activation is resolved
- **THEN** there is no send text

> test: code
> - crates/duckboard/src/obvious_bubble.rs:288

### Scenario: Resolved text ignores oneshot list when both differ

- **GIVEN** visible chrome whose ⌘↩ resolution is action string A
- **AND** a non-empty oneshot default-prompt list whose active entry is B
- **AND** A and B differ
- **WHEN** ⌘↩ activation is resolved
- **THEN** the send text is A
- **AND** the send text is not B

> test: code
> - crates/duckboard/src/obvious_bubble.rs:315

## Requirement: Chip display

Each visible chrome action SHALL present a chip label that places the hotkey glyph and
binding before the action text (lifecycle: `⌘` plus 1-based index; affirm: `⌘↩`; decline:
`⌘⌫`), then the action string. The text sent on activation SHALL be the action string only
— not the hotkey prefix.

> test: code

### Scenario: Lifecycle chip label is hotkey then action

- **GIVEN** a lifecycle option at 1-based index 1 with send text `/ds-step`
- **WHEN** the chip label is derived
- **THEN** the label starts with the ⌘1 hotkey
- **AND** the label includes `/ds-step` after the hotkey
- **AND** the send text is exactly `/ds-step`

> test: code
> - crates/duckboard/src/obvious_bubble.rs:330

### Scenario: Affirm chip label is hotkey then Confirm, Commit, or Create change

- **GIVEN** affirm Create change
- **WHEN** the chip label is derived
- **THEN** the label starts with the ⌘↩ hotkey
- **AND** the label includes `Create change`
- **AND** the send text is exactly `Create change`

> test: code
> - crates/duckboard/src/obvious_bubble.rs:342
