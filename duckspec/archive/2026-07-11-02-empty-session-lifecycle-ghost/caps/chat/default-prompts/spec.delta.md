# @ Chat default prompts

## @ Requirement: Agent input hints gate

A global agent input hints setting SHALL control whether reply-suggestion oneshots run
after turns. The setting SHALL default to disabled. When agent input hints is disabled, a
reply-suggestion oneshot SHALL NOT be started after a non-priming turn completes. When
agent input hints is enabled, oneshot launch follows the non-priming turn rules of this
capability (assistant text present and other launch conditions). There is no separate
auto-messages setting that suppresses oneshots or next-action lists. Empty-session
next-action bootstrap and the next-action list SHALL NOT depend on the agent input hints
setting.

### + Scenario: Empty-session next actions remain when agent input hints disabled

- **GIVEN** agent input hints disabled
- **AND** an empty session transcript
- **AND** a first lifecycle option for that session
- **WHEN** the next-action list is built
- **THEN** the list is exactly that single lifecycle option in empty-send form

> test: code

## @ Requirement: Next-action list

The next-action list for the empty composer SHALL be built as follows. When the session
transcript is empty and a first lifecycle option is present, the list SHALL be exactly
that option in empty-send form (a single entry). When the session transcript is empty and
no first lifecycle option is present, the list SHALL be empty. When the session transcript
is non-empty, the list SHALL be exactly the trailing next actions extracted from the last
non-priming assistant message (via chat meta-card recognition); if that message has no
trailing `next` meta card, the list SHALL be empty. Settled oneshot suggestion strings
SHALL NOT be appended, merged, or substituted into the next-action list. Disk lifecycle
options beyond the empty-session bootstrap SHALL NOT fill the list after the first turn.

For an empty exploration session, the first lifecycle option SHALL be the explore stage
command. For an empty change session, the first lifecycle option SHALL be the first option
of that change's lifecycle ladder from its artifact and step state. Sessions with no
lifecycle ladder (including caps and codex) SHALL have no first lifecycle option from this
bootstrap.

### + Scenario: Empty exploration session seeds explore

- **GIVEN** an empty exploration session transcript
- **WHEN** the next-action list is built
- **THEN** the list is exactly the explore stage command in empty-send form

> test: code

### + Scenario: Empty change session with unfinished steps seeds apply

- **GIVEN** an empty change session transcript
- **AND** that change has at least one unfinished step
- **WHEN** the next-action list is built
- **THEN** the list is exactly the apply stage command in empty-send form

> test: code
