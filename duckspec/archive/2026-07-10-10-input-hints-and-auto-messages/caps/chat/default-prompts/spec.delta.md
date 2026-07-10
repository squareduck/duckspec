# @ Chat default prompts

Under-input input hints for the empty composer: an empty session seeds a single entry from
the first lifecycle option when one exists; a non-empty session uses settled agent oneshot
`REPLY:` suggestions only when the global agent input hints setting is enabled (default
off). Empty Enter and Tab cycle drive that effective list alone.

## @ Requirement: Effective default-prompt list

The effective default-prompt list is built as follows. When the session transcript is
empty and a first lifecycle option is present, the effective list SHALL be exactly that
option in empty-send form (a single entry); the list SHALL NOT wait on a oneshot and SHALL
NOT include oneshot parse results. When the session transcript is empty and no first
lifecycle option is present, the effective list SHALL be empty. When the session
transcript is non-empty and agent input hints are enabled, and a reply-suggestion oneshot
has settled for the current generation with one or more parsed reply strings, the
effective list SHALL be exactly those strings (order preserved, already capped); the first
lifecycle option SHALL NOT be appended or merged into that list. When the session
transcript is non-empty and agent input hints are enabled, and no such non-empty oneshot
result is armed — including a settled oneshot that failed or produced no suggestions — the
effective list SHALL be empty, whether or not a first lifecycle option is present. When
the session transcript is non-empty and agent input hints are disabled, the effective list
SHALL be empty regardless of oneshot storage or lifecycle options. The first lifecycle
option SHALL NOT appear as an effective-list entry for a non-empty session.

> test: code

### ~ Scenario: Parsed replies are the effective list in order

- **GIVEN** a non-empty session transcript
- **AND** agent input hints enabled
- **AND** a settled oneshot whose parse produced three distinct reply strings
- **WHEN** the effective default-prompt list is built
- **THEN** the list is exactly those three strings in parse order

> test: code

### ~ Scenario: No non-empty oneshot result yields an empty list

- **GIVEN** a non-empty session transcript
- **AND** agent input hints enabled
- **AND** no settled non-empty oneshot result
- **WHEN** the effective default-prompt list is built
- **THEN** the list is empty

> test: code

### ~ Scenario: Failed or empty oneshot yields an empty list even with a heuristic

- **GIVEN** a non-empty session transcript
- **AND** agent input hints enabled
- **AND** a settled oneshot that failed or produced no suggestions
- **AND** a present first lifecycle option
- **WHEN** the effective default-prompt list is built
- **THEN** the list is empty

> test: code

### + Scenario: Empty session seeds first lifecycle

- **GIVEN** an empty session transcript
- **AND** a first lifecycle option in empty-send form
- **WHEN** the effective default-prompt list is built
- **THEN** the list is exactly that single lifecycle option

> test: code

### + Scenario: Empty session without lifecycle yields empty

- **GIVEN** an empty session transcript
- **AND** no first lifecycle option
- **WHEN** the effective default-prompt list is built
- **THEN** the list is empty

> test: code

### + Scenario: Non-empty session with agent hints disabled yields empty despite oneshot

- **GIVEN** a non-empty session transcript
- **AND** agent input hints disabled
- **AND** a settled oneshot whose parse produced one or more reply strings
- **WHEN** the effective default-prompt list is built
- **THEN** the list is empty

> test: code

### + Scenario: Empty session ignores oneshot results

- **GIVEN** an empty session transcript
- **AND** a first lifecycle option in empty-send form
- **AND** stored oneshot reply strings that differ from that option
- **WHEN** the effective default-prompt list is built
- **THEN** the list is exactly that single lifecycle option

> test: code

## + Requirement: Agent input hints gate

A global agent input hints setting SHALL control whether reply-suggestion oneshots run
after turns. The setting SHALL default to disabled. When the setting is disabled, a
reply-suggestion oneshot SHALL NOT be started after a non-priming turn completes. When the
setting is enabled, oneshot launch follows the existing non-priming turn rules (assistant
text present, and other launch conditions in this capability).

> test: code

### Scenario: Default agent input hints setting is disabled

- **GIVEN** application config defaults
- **WHEN** the agent input hints setting is read
- **THEN** it is disabled

> test: code

### Scenario: Oneshot launch requires agent input hints enabled

- **GIVEN** agent input hints disabled
- **AND** a non-priming turn that would otherwise qualify for reply suggestions
- **WHEN** oneshot launch is decided
- **THEN** a reply-suggestion oneshot is not started

> test: code
