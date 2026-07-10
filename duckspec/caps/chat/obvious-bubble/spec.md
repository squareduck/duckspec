# Chat obvious bubble

Lifecycle next-command as a greyed faux user bubble and ⌘↩ send path — independent of
oneshot composer suggestions.

## Requirement: Lifecycle send text

When the session has a lifecycle next command, the bubble send text SHALL be that command
in empty-send form: a leading `/` is added when the stored form is a bare skill name (e.g.
`ds-explore` becomes `/ds-explore`); a command that already begins with `/` is kept as
stored. When no lifecycle next command is present, there SHALL be no bubble send text.

> test: code

### Scenario: Bare skill name formats with leading slash

- **GIVEN** a lifecycle next command stored without a leading slash
- **WHEN** the bubble send text is derived
- **THEN** the send text is that command with a single leading `/`

> test: code
> - crates/duckboard/src/obvious_bubble.rs:52

### Scenario: Already-slashed command is preserved

- **GIVEN** a lifecycle next command that already begins with `/`
- **WHEN** the bubble send text is derived
- **THEN** the send text equals the stored command

> test: code
> - crates/duckboard/src/obvious_bubble.rs:68

### Scenario: Absent command yields no send text

- **GIVEN** no lifecycle next command for the session
- **WHEN** the bubble send text is derived
- **THEN** there is no send text

> test: code
> - crates/duckboard/src/obvious_bubble.rs:80

## Requirement: Bubble visibility

The obvious bubble SHALL be shown only when all of the following hold: the main agent turn
is not in progress, the composer input is empty, and a lifecycle next command is present.
A pending or settled oneshot for composer default prompts SHALL NOT hide the bubble when
those gates hold. The bubble SHALL NOT be shown when any gate fails — no lifecycle
command, non-empty composer, or main turn in progress.

> test: code

### Scenario: Idle empty composer with command shows bubble

- **GIVEN** a lifecycle next command for the session
- **AND** an empty composer input
- **AND** no main agent turn in progress
- **WHEN** bubble visibility is evaluated
- **THEN** the bubble is shown

> test: code
> - crates/duckboard/src/obvious_bubble.rs:91

### Scenario: Streaming hides bubble

- **GIVEN** a lifecycle next command for the session
- **AND** an empty composer input
- **AND** a main agent turn in progress
- **WHEN** bubble visibility is evaluated
- **THEN** the bubble is not shown

> test: code
> - crates/duckboard/src/obvious_bubble.rs:100

### Scenario: Non-empty composer hides bubble

- **GIVEN** a lifecycle next command for the session
- **AND** a non-empty composer input
- **AND** no main agent turn in progress
- **WHEN** bubble visibility is evaluated
- **THEN** the bubble is not shown

> test: code
> - crates/duckboard/src/obvious_bubble.rs:109

### Scenario: No command hides bubble

- **GIVEN** no lifecycle next command for the session
- **AND** an empty composer input
- **AND** no main agent turn in progress
- **WHEN** bubble visibility is evaluated
- **THEN** the bubble is not shown

> test: code
> - crates/duckboard/src/obvious_bubble.rs:118

### Scenario: Oneshot pending does not hide bubble when otherwise visible

- **GIVEN** a lifecycle next command for the session
- **AND** an empty composer input
- **AND** no main agent turn in progress
- **AND** a pending reply-suggestion oneshot
- **WHEN** bubble visibility is evaluated
- **THEN** the bubble is shown

> test: code
> - crates/duckboard/src/obvious_bubble.rs:129

## Requirement: Activation send

When the bubble is visible, activating it (⌘↩ or bubble activation) SHALL send the bubble
send text as a normal user message on the same path as a typed submit of that text.
Activation SHALL NOT send a message when the bubble is not visible. The text sent SHALL be
the lifecycle bubble send text only — it SHALL NOT be taken from the oneshot
default-prompt list, even when that list is non-empty and differs from the lifecycle
command.

> test: code

### Scenario: Activation sends lifecycle text when visible

- **GIVEN** the obvious bubble is visible
- **AND** a derived bubble send text
- **WHEN** the bubble is activated
- **THEN** a user message is sent whose text is the bubble send text

> test: code
> - crates/duckboard/src/obvious_bubble.rs:140

### Scenario: Activation is a no-op when not visible

- **GIVEN** the obvious bubble is not visible
- **WHEN** bubble activation is requested
- **THEN** no message is sent

> test: code
> - crates/duckboard/src/obvious_bubble.rs:150

### Scenario: Send text ignores oneshot list when both differ

- **GIVEN** the obvious bubble is visible with lifecycle send text A
- **AND** a non-empty oneshot default-prompt list whose active entry is B
- **AND** A and B differ
- **WHEN** the bubble is activated
- **THEN** the sent text is A
- **AND** the sent text is not B

> test: code
> - crates/duckboard/src/obvious_bubble.rs:161

## Requirement: Ephemeral chrome

The obvious bubble SHALL NOT be stored in the session transcript until activation produces
a real user message. While only shown as chrome, it SHALL NOT appear as a committed user
message in the session.

> test: code

### Scenario: Visible bubble is not a stored user message

- **GIVEN** the obvious bubble is shown
- **AND** it has not been activated
- **WHEN** the session transcript is inspected
- **THEN** it does not contain a user message whose sole purpose is the ghost bubble

> test: code
> - crates/duckboard/src/obvious_bubble.rs:176
