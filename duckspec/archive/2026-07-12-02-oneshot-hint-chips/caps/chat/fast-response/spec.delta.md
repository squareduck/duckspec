# @ Chat fast response

Source-neutral option chips for mid-turn structured choices and settled oneshot reply
suggestions: ordered options with ⌘-number activation, ephemeral view layout, and
empty-send formatting for bare skill names. A live user-choice request fills the shell for
in-band answers; settled oneshot hints may fill it for ordinary user-message sends when
eligible. Freeform composer submit while awaiting completes the pending choice as a custom
answer. While awaiting, the composer section uses a quiet accent tint, including the model
selector.

## @ Requirement: Visibility

Fast-response chips SHALL be shown only when all of the following hold: the shell has at
least one option; either no main agent turn is in progress, or the session is awaiting a
user choice for an open turn; and either the composer input is empty, or the session is
awaiting a user choice. While awaiting a user choice, a non-empty composer SHALL NOT hide
the chips (composer is the custom-answer surface). When the session is not awaiting a user
choice, a non-empty composer SHALL hide the chips. The chips SHALL NOT be shown when any
gate fails.

## @ Requirement: Population

For ordinary change, exploration, caps, and codex chat sessions, fast-response options
SHALL be empty after a refresh when the session is not awaiting a user choice and oneshot
replies are not eligible to fill chips. While the session is awaiting a user choice with
non-empty options, a refresh SHALL NOT clear those options. When oneshot replies are
eligible, a refresh SHALL re-sync the shell from the settled oneshot list (non-empty
options with oneshot-hint authority). A live mid-turn user-choice request SHALL fill the
shell from that choice and SHALL overwrite any oneshot-hint fill. A settled oneshot result
SHALL NOT replace the shell while the session is awaiting a user choice.

> test: code

### - Scenario: Ordinary refresh leaves options empty when not awaiting a choice

### + Scenario: Ordinary refresh leaves options empty when oneshot is ineligible

- **GIVEN** a change or exploration session that is not awaiting a user choice
- **AND** oneshot replies are not eligible to fill chips
- **WHEN** fast response is refreshed for that session
- **THEN** the options list is empty

> test: code

### + Scenario: Refresh preserves oneshot fill when still eligible

- **GIVEN** a session that is not awaiting a user choice
- **AND** oneshot replies are eligible with a non-empty settled list
- **AND** the shell is filled from that oneshot list
- **WHEN** fast response is refreshed for that session
- **THEN** the options list remains non-empty
- **AND** the options match the settled oneshot list in order

> test: code

### + Scenario: Settled eligible oneshot fills the option shell

- **GIVEN** a session that is not awaiting a user choice
- **AND** oneshot replies are eligible with a non-empty settled list
- **WHEN** the oneshot shell is synced
- **THEN** the options list contains those settled replies in order

> test: code

### + Scenario: Live user choice overwrites oneshot fill

- **GIVEN** a shell filled from settled oneshot hints
- **AND** a mid-turn user-choice request with at least one option
- **WHEN** that user-choice request is applied
- **THEN** the options list matches the user-choice options
- **AND** the shell is no longer filled from oneshot hints

> test: code

### + Scenario: Oneshot settle does not replace a live user-choice fill

- **GIVEN** a session awaiting a user choice with non-empty fast-response options
- **AND** a settled oneshot list that would be eligible if not awaiting
- **WHEN** the oneshot shell is synced
- **THEN** the options list remains the user-choice options

> test: code

## + Requirement: Oneshot activation

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
