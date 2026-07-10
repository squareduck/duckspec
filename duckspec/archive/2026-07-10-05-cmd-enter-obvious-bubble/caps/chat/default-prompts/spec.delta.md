# @ Chat default prompts

Conversation-local empty-input defaults from a cheap-model oneshot: parse ordered `REPLY:`
suggestions (lifecycle heuristic passed only as a soft request hint), show and arm them
only after a non-empty parse settles, and drive empty Enter plus Tab cycling from that
list alone. The effective list is never seeded or filled from the lifecycle heuristic.

## @ Requirement: Effective default-prompt list

The effective default-prompt list is built as follows. When a reply-suggestion oneshot has
settled for the current generation with one or more parsed reply strings, the effective
list SHALL be exactly those strings (order preserved, already capped); the lifecycle
heuristic SHALL NOT be appended or merged into that list. When no such non-empty oneshot
result is armed — including a brand-new session that has never run a oneshot, and a
settled oneshot that failed or produced no suggestions — the effective list SHALL be
empty, whether or not a lifecycle heuristic is present. The lifecycle heuristic SHALL NOT
appear as an effective-list entry.

> test: code

### - Scenario: Pre-oneshot list is the lifecycle heuristic when present

### - Scenario: Failed or empty oneshot falls back to the heuristic

### - Scenario: No oneshot and no heuristic yields an empty list

### + Scenario: No non-empty oneshot result yields an empty list

- **GIVEN** a session with no settled non-empty oneshot result
- **WHEN** the effective default-prompt list is built
- **THEN** the list is empty

> test: code

### + Scenario: Failed or empty oneshot yields an empty list even with a heuristic

- **GIVEN** a settled oneshot that failed or produced no suggestions
- **AND** a present lifecycle heuristic
- **WHEN** the effective default-prompt list is built
- **THEN** the list is empty

> test: code

## @ Requirement: Suggestion readiness

### ~ Scenario: Timed-out or failed oneshot settles to ready

- **GIVEN** a pending reply-suggestion oneshot
- **AND** that oneshot settles as a failure for the current generation
- **AND** an empty composer input
- **WHEN** the empty-input defaults chrome is rendered
- **THEN** a loading indicator is not shown
- **AND** suggestions are ready
- **AND** the effective list is empty when the failure produced no parse

> test: code
