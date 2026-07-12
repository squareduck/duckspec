# @ Chat default prompts

Empty-composer next actions from an inherited donor list (empty change sessions after
new-session creation), lifecycle bootstrap, or a trailing `next` meta card, shown as ghost
text with empty Enter and Tab cycle; optional settings-gated oneshot reply suggestions (up
to three freeform `REPLY:` lines) that may fill fast-response chips only when there is no
next-action ghost.

## @ Requirement: Next-action list

The next-action list for the empty composer SHALL be built as follows. When the session
transcript is empty and a non-empty inherited next-action list is present, the list SHALL
be exactly that inherited list (in order). When the session transcript is empty, no
non-empty inherited list is present, and a first lifecycle option is present, the list
SHALL be exactly that option in empty-send form (a single entry). When the session
transcript is empty and neither a non-empty inherited list nor a first lifecycle option is
present, the list SHALL be empty. When the session transcript is non-empty, the list SHALL
be exactly the trailing next actions extracted from the last non-priming assistant message
(via chat meta-card recognition); if that message has no trailing `next` meta card, the
list SHALL be empty. A non-empty transcript SHALL NOT use an inherited next-action list.
Settled oneshot suggestion strings SHALL NOT be appended, merged, or substituted into the
next-action list. Disk lifecycle options beyond the empty-session bootstrap SHALL NOT fill
the list after the first turn.

For an empty exploration session, the first lifecycle option SHALL be the explore stage
command. For an empty change session, the first lifecycle option SHALL be the first option
of that change's lifecycle ladder from its artifact and step state. Sessions with no
lifecycle ladder (including caps and codex) SHALL have no first lifecycle option from this
bootstrap.

### + Scenario: Empty session with inherited next actions uses inherited list

- **GIVEN** an empty session transcript
- **AND** a non-empty inherited next-action list of two distinct send tokens
- **AND** a first lifecycle option that differs from those tokens
- **WHEN** the next-action list is built
- **THEN** the list is exactly those two inherited send tokens in order

> test: code

### + Scenario: Empty session without inherited falls back to lifecycle

- **GIVEN** an empty session transcript
- **AND** no non-empty inherited next-action list
- **AND** a first lifecycle option in empty-send form
- **WHEN** the next-action list is built
- **THEN** the list is exactly that single lifecycle option

> test: code

### + Scenario: Non-empty session drops inheritance

- **GIVEN** a non-empty session transcript

- **AND** a non-empty inherited next-action list

- **AND** a last non-priming assistant message whose trailing next actions are a single
  send token that differs from the inherited list

- **WHEN** the next-action list is built

- **THEN** the list is exactly that trailing next send token

- **AND** the inherited list is not used

> test: code

## + Requirement: New-session next-action inheritance

When a new change chat session is created for multi-session change chat, if the session
that was active at creation has a non-empty next-action list, the new empty session SHALL
receive that list as its inherited next-action list and its next-action list SHALL match
those send tokens in order. The active next action on the new session SHALL be the first
entry. When the active session's next-action list is empty at creation, the new empty
session SHALL NOT receive an inherited list and SHALL follow empty-session next-action
list rules without inheritance. Inheritance applies only while the new session transcript
remains empty. Oneshot reply suggestions SHALL NOT be copied onto the new session by this
path.

### Scenario: New change session inherits active session next actions

- **GIVEN** a change scope with multi-session chat
- **AND** the active session has a non-empty next-action list of two distinct send tokens
- **WHEN** a new chat session is created for that change
- **THEN** the new session's next-action list is exactly those two send tokens in order
- **AND** the new session transcript is empty

> test: code

### Scenario: New change session with empty donor keeps bootstrap behavior

- **GIVEN** a change scope with multi-session chat
- **AND** the active session has an empty next-action list
- **AND** a first lifecycle option for that change in empty-send form
- **WHEN** a new chat session is created for that change
- **THEN** the new session's next-action list is exactly that single lifecycle option

> test: code

### Scenario: Inherited list starts at first action

- **GIVEN** a change scope with multi-session chat
- **AND** the active session has a next-action list of two or more send tokens
- **AND** the active session's active next-action index is not the first entry
- **WHEN** a new chat session is created for that change
- **THEN** empty submit on the new session sends the first inherited send token

> test: code
