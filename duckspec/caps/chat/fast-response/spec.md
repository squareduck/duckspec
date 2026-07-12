# Chat fast response

Source-neutral option chips for mid-turn structured choices: ordered options with ⌘-number
activation, ephemeral view layout, and empty-send formatting for bare skill names.
Ordinary refresh leaves the shell empty; a live user-choice request may fill it. Chip
activation answers in-band and does not invent a user message. Freeform composer submit
while awaiting completes the pending choice as a custom answer (not cancel + next turn).
While awaiting, the composer section uses a quiet accent tint, including the model
selector.

Source-neutral option chips for mid-turn structured choices and settled oneshot reply
suggestions: ordered options with ⌘-number activation, ephemeral view layout, and
empty-send formatting for bare skill names. A live user-choice request fills the shell for
in-band answers; settled oneshot hints may fill it for ordinary user-message sends when
eligible. Freeform composer submit while awaiting completes the pending choice as a custom
answer. While awaiting, the composer section uses a quiet accent tint, including the model
selector.

## Requirement: Ephemeral chips

Fast-response chips SHALL NOT be stored in the session transcript as committed user
messages. While only shown as view chrome, they SHALL NOT appear as user messages in the
session.

> test: code

### Scenario: Visible chips are not a stored user message

- **GIVEN** fast-response chips are shown
- **AND** no chip action has been activated
- **WHEN** the session transcript is inspected
- **THEN** it does not contain a user message whose sole purpose is a fast-response chip

> test: code
> - crates/duckboard/src/fast_response.rs:317

## Requirement: Visibility

Fast-response chips SHALL be shown only when all of the following hold: the shell has at
least one option; either no main agent turn is in progress, or the session is awaiting a
user choice for an open turn; and either the composer input is empty, or the session is
awaiting a user choice. While awaiting a user choice, a non-empty composer SHALL NOT hide
the chips (composer is the custom-answer surface). When the session is not awaiting a user
choice, a non-empty composer SHALL hide the chips. The chips SHALL NOT be shown when any
gate fails.

### Scenario: Idle empty composer with options shows chips

- **GIVEN** non-empty fast-response options for the session
- **AND** an empty composer input
- **AND** no main agent turn in progress
- **WHEN** visibility is evaluated
- **THEN** the chips are shown

> test: code
> - crates/duckboard/src/fast_response.rs:215

### Scenario: Streaming without awaiting user hides chips

- **GIVEN** non-empty fast-response options for the session
- **AND** an empty composer input
- **AND** a main agent turn in progress
- **AND** the session is not awaiting a user choice
- **WHEN** visibility is evaluated
- **THEN** the chips are not shown

> test: code
> - crates/duckboard/src/fast_response.rs:222

### Scenario: Awaiting user shows chips while turn is open

- **GIVEN** non-empty fast-response options for the session
- **AND** an empty composer input
- **AND** a main agent turn in progress
- **AND** the session is awaiting a user choice
- **WHEN** visibility is evaluated
- **THEN** the chips are shown

> test: code
> - crates/duckboard/src/fast_response.rs:229

### Scenario: Non-empty composer hides chips when not awaiting

- **GIVEN** non-empty fast-response options for the session
- **AND** a non-empty composer input
- **AND** the session is not awaiting a user choice
- **WHEN** visibility is evaluated
- **THEN** the chips are not shown

> test: code
> - crates/duckboard/src/fast_response.rs:237

### Scenario: Awaiting user shows chips with non-empty composer

- **GIVEN** non-empty fast-response options for the session
- **AND** a non-empty composer input
- **AND** the session is awaiting a user choice
- **WHEN** visibility is evaluated
- **THEN** the chips are shown

> test: code
> - crates/duckboard/src/fast_response.rs:245

### Scenario: Empty options hide chips

- **GIVEN** empty fast-response options
- **AND** an empty composer input
- **AND** no main agent turn in progress
- **WHEN** visibility is evaluated
- **THEN** the chips are not shown

> test: code
> - crates/duckboard/src/fast_response.rs:254

## Requirement: Key resolution

When the chips are visible, key activation SHALL resolve as follows: ⌘*n* for digit *n*
from 1 through 9 yields the *n*th option when that index exists, otherwise no selection.
When the chips are not visible, every such activation SHALL be a no-op. The resolved
selection SHALL be the option payload only — not a hotkey prefix and not an under-input
oneshot suggestion. There is no cancel key binding on the shell.

> test: code

### Scenario: Cmd-digit selects matching option

- **GIVEN** visible chips with at least two options
- **WHEN** ⌘2 activation is resolved
- **THEN** the selection is the second option

> test: code
> - crates/duckboard/src/fast_response.rs:262

### Scenario: Resolution is a no-op when chips not visible

- **GIVEN** chips that are not visible
- **WHEN** ⌘1 activation is resolved
- **THEN** there is no selection

> test: code
> - crates/duckboard/src/fast_response.rs:276

## Requirement: Chip labels

Each visible option SHALL present a chip label that places the hotkey glyph and binding
before the action text: `⌘` plus 1-based index then the option label. The payload used on
activation SHALL be the option identity only — not the hotkey prefix. The shell does not
present a cancel chip.

> test: code

### Scenario: Option chip label is hotkey then action

- **GIVEN** an option at 1-based index 1 with label text `/ds-step`
- **WHEN** the chip label is derived
- **THEN** the label starts with the ⌘1 hotkey
- **AND** the label includes `/ds-step` after the hotkey

> test: code
> - crates/duckboard/src/fast_response.rs:295

## Requirement: Bottom pad

When fast-response chips are visible, they SHALL render inside the chat scroll column
after transcript content (not between the scroll viewport and the composer). A top pad
above the chips SHALL be derived so chips sit at the bottom of the chat viewport when
natural content is shorter than the viewport. Given viewport height `viewport_h`, laid-out
scroll content height including any previous pad `content_h`, and the previous pad height
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
- **WHEN** the bottom pad is derived
- **THEN** the pad height is 300

> test: code
> - crates/duckboard/src/fast_response.rs:305

### Scenario: Content at or above viewport yields zero pad

- **GIVEN** a viewport height of 400
- **AND** a content height of 500 including a previous pad of 0
- **WHEN** the bottom pad is derived
- **THEN** the pad height is 0

> test: code
> - crates/duckboard/src/fast_response.rs:311

## Requirement: Population

For ordinary change, exploration, caps, and codex chat sessions, fast-response options
SHALL be empty after a refresh when the session is not awaiting a user choice and oneshot
replies are not eligible to fill chips. While the session is awaiting a user choice with
non-empty options, a refresh SHALL NOT clear those options. When oneshot replies are
eligible, a refresh SHALL re-sync the shell from the settled oneshot list (non-empty
options with oneshot-hint authority). A live mid-turn user-choice request SHALL fill the
shell from that choice and SHALL overwrite any oneshot-hint fill. A settled oneshot result
SHALL NOT replace the shell while the session is awaiting a user choice.

> test: code

### Scenario: Refresh does not clear options while awaiting a user choice

- **GIVEN** a session awaiting a user choice with non-empty fast-response options
- **WHEN** fast response is refreshed for that session
- **THEN** the options list remains non-empty

> test: code
> - crates/duckboard/src/area/change.rs:2278
> - crates/duckboard/src/fast_response.rs:340

### Scenario: Ordinary refresh leaves options empty when oneshot is ineligible

- **GIVEN** a change or exploration session that is not awaiting a user choice
- **AND** oneshot replies are not eligible to fill chips
- **WHEN** fast response is refreshed for that session
- **THEN** the options list is empty

> test: code
> - crates/duckboard/src/area/change.rs:2238

### Scenario: Refresh preserves oneshot fill when still eligible

- **GIVEN** a session that is not awaiting a user choice
- **AND** oneshot replies are eligible with a non-empty settled list
- **AND** the shell is filled from that oneshot list
- **WHEN** fast response is refreshed for that session
- **THEN** the options list remains non-empty
- **AND** the options match the settled oneshot list in order

> test: code
> - crates/duckboard/src/area/change.rs:2317

### Scenario: Settled eligible oneshot fills the option shell

- **GIVEN** a session that is not awaiting a user choice
- **AND** oneshot replies are eligible with a non-empty settled list
- **WHEN** the oneshot shell is synced
- **THEN** the options list contains those settled replies in order

> test: code
> - crates/duckboard/src/area/change.rs:2357

### Scenario: Live user choice overwrites oneshot fill

- **GIVEN** a shell filled from settled oneshot hints
- **AND** a mid-turn user-choice request with at least one option
- **WHEN** that user-choice request is applied
- **THEN** the options list matches the user-choice options
- **AND** the shell is no longer filled from oneshot hints

> test: code
> - crates/duckboard/src/area/change.rs:2385

### Scenario: Oneshot settle does not replace a live user-choice fill

- **GIVEN** a session awaiting a user choice with non-empty fast-response options
- **AND** a settled oneshot list that would be eligible if not awaiting
- **WHEN** the oneshot shell is synced
- **THEN** the options list remains the user-choice options

> test: code
> - crates/duckboard/src/area/change.rs:2417

## Requirement: Question activation

When the shell is filled from a pending mid-turn user choice and chips are visible,
activating an option SHALL complete that choice with the selected option and SHALL NOT
append a new user message to the session transcript for that activation.

> test: code

### Scenario: Option activation answers the pending choice without a new user message

- **GIVEN** visible chips filled from a pending user choice with at least one option
- **WHEN** the first option is activated
- **THEN** the pending choice is completed with that option
- **AND** the session transcript does not gain a new user message for the activation

> test: code
> - crates/duckboard/src/fast_response.rs:362

## Requirement: Freeform while awaiting

When the session is awaiting a user choice and the user submits non-empty freeform
composer text, the system SHALL complete that pending choice as a **custom answer** whose
payload is that freeform text (so the harness finishes the question tool with free text as
the answer value, not as cancel/skip). It SHALL clear the choice shell. It SHALL NOT leave
the text only staged in the interrupt queue, and SHALL NOT invent a separate user
transcript message solely for that custom answer.

> test: code

### Scenario: Freeform submit completes the pending choice as a custom answer

- **GIVEN** a session awaiting a user choice with non-empty fast-response options

- **AND** non-empty freeform text in the composer

- **WHEN** the user submits the composer

- **THEN** the pending choice is completed as a custom answer carrying that freeform text

- **AND** the session transcript does not gain a new user message solely for that custom
  answer

- **AND** the text is not left only staged in the interrupt queue

> test: code
> - crates/duckboard/src/area/interaction.rs:2039

## Requirement: Awaiting composer chrome

While the session is awaiting a user choice, the chat composer section (input and its
footer strip, including the model selector) SHALL present a quiet accent tint matching the
numbered option-chip treatment so the strip reads as the custom-answer surface. When the
session is not awaiting a user choice, that section SHALL NOT use that awaiting tint. The
model selector surface SHALL match the surrounding composer section tint while awaiting so
it does not stand out as an untinted control.

> test: code

### Scenario: Awaiting user applies quiet accent tint to the composer section

- **GIVEN** a session awaiting a user choice
- **WHEN** the composer section styles are derived
- **THEN** the composer section uses the quiet accent awaiting tint

> test: code
> - crates/duckboard/src/fast_response.rs:413

### Scenario: Not awaiting leaves the composer section untinted

- **GIVEN** a session that is not awaiting a user choice
- **WHEN** the composer section styles are derived
- **THEN** the composer section does not use the quiet accent awaiting tint

> test: code
> - crates/duckboard/src/fast_response.rs:430

### Scenario: Model selector matches the composer section tint while awaiting

- **GIVEN** a session awaiting a user choice
- **WHEN** the model selector style is derived for the composer footer
- **THEN** it uses the same quiet accent awaiting tint as the composer section

> test: code
> - crates/duckboard/src/fast_response.rs:442

## Requirement: Empty-send formatting

When a bare skill name is formatted for empty-send use, the result SHALL be that name with
a single leading `/` (e.g. `ds-explore` becomes `/ds-explore`). A name that already begins
with `/` SHALL be kept as stored. Empty or blank skill names SHALL not produce a send
string.

> test: code

### Scenario: Bare skill name formats with leading slash

- **GIVEN** a skill name stored without a leading slash
- **WHEN** the empty-send text is derived
- **THEN** the send text is that name with a single leading `/`

> test: code
> - crates/duckboard/src/fast_response.rs:193

### Scenario: Already-slashed command is preserved

- **GIVEN** a skill name that already begins with `/`
- **WHEN** the empty-send text is derived
- **THEN** the send text equals the stored name

> test: code
> - crates/duckboard/src/fast_response.rs:206

## Requirement: Oneshot activation

When the shell is filled from settled oneshot reply hints and chips are visible,
activating an option SHALL send that option's text as a normal user message on the
session. It SHALL NOT complete a mid-turn user choice in-band for that activation.

> test: code

### Scenario: Option activation sends the oneshot text as a user message

- **GIVEN** visible chips filled from settled oneshot hints with at least one option
- **WHEN** the first option is activated
- **THEN** a new user message is sent whose text is that option's text
- **AND** no mid-turn user choice is completed in-band for the activation

> test: code
> - crates/duckboard/src/fast_response.rs:391
