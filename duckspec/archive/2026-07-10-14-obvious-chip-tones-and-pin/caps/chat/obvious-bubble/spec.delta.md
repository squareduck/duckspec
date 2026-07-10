# @ Chat obvious bubble

## ~ Requirement: Chip display

Each visible chrome action SHALL present a chip label that places the hotkey glyph and
binding before the action text (lifecycle: `⌘` plus 1-based index; affirm: `⌘↩`; decline:
`⌘⌫`), then the action string. The text sent on activation SHALL be the action string only
— not the hotkey prefix.

When the chrome has more than one lifecycle option and no affirm, the first lifecycle
option SHALL be dual-presented: once as its numbered lifecycle chip (hotkey plus
empty-send `/ds-…` text) among the ordered lifecycle chips, and once as a separate enter
chip after all lifecycle chips whose label uses the `⌘↩` hotkey followed by a friendly
name derived from that option. The enter dual chip's send text SHALL be the original first
lifecycle option string, not the friendly name. Friendly names SHALL strip a leading
`/ds-` or `ds-` prefix when present and title-case the remainder (e.g. `/ds-apply` yields
`Apply`).

When the chrome has exactly one lifecycle option and no affirm, or when affirm is present,
the first lifecycle option SHALL NOT be dual-presented as a separate enter chip.

> test: code

### Scenario: Lifecycle chip label is hotkey then action

- **GIVEN** a lifecycle option at 1-based index 1 with send text `/ds-step`
- **WHEN** the chip label is derived
- **THEN** the label starts with the ⌘1 hotkey
- **AND** the label includes `/ds-step` after the hotkey
- **AND** the send text is exactly `/ds-step`

> test: code

### Scenario: Affirm chip label is hotkey then Confirm, Commit, or Create change

- **GIVEN** affirm Create change
- **WHEN** the chip label is derived
- **THEN** the label starts with the ⌘↩ hotkey
- **AND** the label includes `Create change`
- **AND** the send text is exactly `Create change`

> test: code

### Scenario: Multi lifecycle without affirm dual-presents first option

- **GIVEN** chrome with two or more lifecycle options
- **AND** no affirm
- **WHEN** dual-enter presentation is derived
- **THEN** dual-enter is active for the first lifecycle option
- **AND** that option retains its numbered lifecycle chip label

> test: code

### Scenario: Single lifecycle does not dual-present

- **GIVEN** chrome with exactly one lifecycle option
- **AND** no affirm
- **WHEN** dual-enter presentation is derived
- **THEN** dual-enter is not active

> test: code

### Scenario: Affirm present does not dual-present lifecycle

- **GIVEN** chrome with one or more lifecycle options
- **AND** affirm is present
- **WHEN** dual-enter presentation is derived
- **THEN** dual-enter is not active

> test: code

### Scenario: Enter dual label is hotkey then friendly name with original send text

- **GIVEN** a first lifecycle option `/ds-apply`
- **AND** dual-enter is active
- **WHEN** the enter dual chip label and send text are derived
- **THEN** the label starts with the ⌘↩ hotkey
- **AND** the label includes `Apply` after the hotkey
- **AND** the label does not include `/ds-apply` as the action text
- **AND** the send text is exactly `/ds-apply`

> test: code

## + Requirement: Chrome bottom pad

When obvious chrome is visible in the chat scroll column, a top pad above the chrome SHALL
be derived so chips sit at the bottom of the chat viewport when natural content is shorter
than the viewport. Given viewport height `viewport_h`, laid-out scroll content height
including any previous pad `content_h`, and the previous pad height `prev_pad`, the pad
height SHALL be `max(0, viewport_h - (content_h - prev_pad))`. When natural content height
(content without the previous pad) is greater than or equal to the viewport height, the
pad SHALL be zero. The pad is view layout only and SHALL NOT be stored in the session
transcript.

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
