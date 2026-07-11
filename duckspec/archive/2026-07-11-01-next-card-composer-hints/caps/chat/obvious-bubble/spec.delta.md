# @ Chat obvious bubble

Generic empty-composer option chrome: ordered option chips with ⌘-number send, optional
cancel on ⌘⌫, ephemeral view layout, and empty-send formatting for bare skill names — not
populated by disk lifecycle or auto-messages in this capability's product path.

## = Requirement: Lifecycle option formatting

Requirement: Empty-send option formatting

## - Requirement: Chrome composition

## ~ Requirement: Empty-send option formatting

When an option is derived from a bare skill name, its send text SHALL be that name in
empty-send form with a single leading `/` (e.g. `ds-explore` becomes `/ds-explore`). An
option that already begins with `/` SHALL be kept as stored. Empty or blank skill names
SHALL not produce a send string.

> test: code

### Scenario: Bare skill name formats with leading slash

- **GIVEN** a skill name stored without a leading slash
- **WHEN** the empty-send text is derived
- **THEN** the send text is that name with a single leading `/`

> test: code

### Scenario: Already-slashed command is preserved

- **GIVEN** a skill name that already begins with `/`
- **WHEN** the empty-send text is derived
- **THEN** the send text equals the stored name

> test: code

## ~ Requirement: Ephemeral chrome

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

## ~ Requirement: Chrome visibility

Obvious chrome SHALL be shown only when all of the following hold: the main agent turn is
not in progress, the composer input is empty, and the chrome is non-empty (at least one
option or a cancel action). There is no auto-messages setting that gates chrome
visibility. A pending or settled oneshot for under-input reply suggestions SHALL NOT hide
the chrome when those gates hold. The chrome SHALL NOT be shown when any gate fails.

> test: code

### Scenario: Idle empty composer with non-empty options shows chrome

- **GIVEN** non-empty obvious chrome options for the session
- **AND** an empty composer input
- **AND** no main agent turn in progress
- **WHEN** chrome visibility is evaluated
- **THEN** the chrome is shown

> test: code

### Scenario: Streaming hides chrome

- **GIVEN** non-empty obvious chrome options for the session
- **AND** an empty composer input
- **AND** a main agent turn in progress
- **WHEN** chrome visibility is evaluated
- **THEN** the chrome is not shown

> test: code

### Scenario: Non-empty composer hides chrome

- **GIVEN** non-empty obvious chrome options for the session
- **AND** a non-empty composer input
- **AND** no main agent turn in progress
- **WHEN** chrome visibility is evaluated
- **THEN** the chrome is not shown

> test: code

### Scenario: Empty options hide chrome

- **GIVEN** empty obvious chrome options and no cancel action
- **AND** an empty composer input
- **AND** no main agent turn in progress
- **WHEN** chrome visibility is evaluated
- **THEN** the chrome is not shown

> test: code

## ~ Requirement: Key resolution

When the chrome is visible, key activation SHALL resolve to a send string as follows: ⌘*n*
for digit *n* from 1 through 9 yields the *n*th option when that index exists, otherwise
no send; ⌘⌫ yields the cancel send text when cancel is set, otherwise no send. When the
chrome is not visible, every such activation SHALL be a no-op (no send). The resolved send
text SHALL be the option or cancel string only — not a hotkey prefix and not the
under-input oneshot suggestion.

> test: code

### Scenario: Cmd-digit sends matching option

- **GIVEN** visible chrome with at least two options
- **WHEN** ⌘2 activation is resolved
- **THEN** the send text equals the second option

> test: code

### Scenario: Cmd-Backspace sends cancel when set

- **GIVEN** visible chrome with cancel set to a non-empty send string
- **WHEN** ⌘⌫ activation is resolved
- **THEN** the send text equals that cancel string

> test: code

### Scenario: Resolution is a no-op when chrome not visible

- **GIVEN** chrome that is not visible
- **WHEN** ⌘⌫ or ⌘1 activation is resolved
- **THEN** there is no send text

> test: code

## ~ Requirement: Chip display

Each visible option SHALL present a chip label that places the hotkey glyph and binding
before the action text: `⌘` plus 1-based index then the option send text. When cancel is
set, a cancel chip SHALL place `⌘⌫` before the cancel send text. The text sent on
activation SHALL be the action string only — not the hotkey prefix.

> test: code

### Scenario: Option chip label is hotkey then action

- **GIVEN** an option at 1-based index 1 with send text `/ds-step`
- **WHEN** the chip label is derived
- **THEN** the label starts with the ⌘1 hotkey
- **AND** the label includes `/ds-step` after the hotkey
- **AND** the send text is exactly `/ds-step`

> test: code

### Scenario: Cancel chip label is hotkey then cancel text

- **GIVEN** cancel send text `cancel`
- **WHEN** the cancel chip label is derived
- **THEN** the label starts with the ⌘⌫ hotkey
- **AND** the label includes `cancel`
- **AND** the send text is exactly `cancel`

> test: code

## ~ Requirement: Chrome bottom pad

When obvious chrome is visible, chips SHALL render inside the chat scroll column after
transcript content (not between the scroll viewport and the composer). A top pad above the
chrome SHALL be derived so chips sit at the bottom of the chat viewport when natural
content is shorter than the viewport. Given viewport height `viewport_h`, laid-out scroll
content height including any previous pad `content_h`, and the previous pad height
`prev_pad`, the pad height SHALL be `max(0, viewport_h - (content_h - prev_pad))`. When
natural content height (content without the previous pad) is greater than or equal to the
viewport height, the pad SHALL be zero and chips follow the last message in document
order. The pad is view layout only and SHALL NOT be stored in the session transcript. Pad
measurement SHALL work even when content fits the viewport (scroll notifications alone are
not sufficient).

> test: code

### Scenario: Short content yields positive pad

- **GIVEN** a viewport height of 400
- **AND** a content height of 100 including a previous pad of 0
- **WHEN** the chrome bottom pad is derived
- **THEN** the pad height is 300

> test: code

### Scenario: Content at or above viewport yields zero pad

- **GIVEN** a viewport height of 400
- **AND** a content height of 500 including a previous pad of 0
- **WHEN** the chrome bottom pad is derived
- **THEN** the pad height is 0

> test: code

## + Requirement: Chrome population

For ordinary change, exploration, caps, and codex chat sessions, obvious chrome options
and cancel SHALL be empty after chrome refresh — the product path does not compose
lifecycle phase chips, affirm rows, or decline rows into chrome. A later path MAY fill
options and cancel; until then, chrome remains empty and therefore not shown.

> test: code

### Scenario: Session chrome options are empty after refresh

- **GIVEN** a change or exploration session that would previously have produced lifecycle
  chips

- **WHEN** obvious chrome is refreshed for that session

- **THEN** the chrome options list is empty

- **AND** cancel is not set

> test: code
