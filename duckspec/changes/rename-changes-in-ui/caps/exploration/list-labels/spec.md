# Exploration list labels

Manual rename and on-demand AI retitle for exploration rows in the CHANGE list, so labels
stay under user control after the first auto-title.

## Requirement: Manual rename updates the exploration label

When the user commits a non-empty rename for an exploration in the CHANGE list, the system
SHALL set that exploration's display name to the committed text and SHALL persist the new
name so a later load of explorations shows it. A blank or whitespace-only commit SHALL
leave the existing display name unchanged.

> test: code

### Scenario: Non-empty rename replaces the list label and persists

- **GIVEN** an exploration whose display name is `Exploration 3`

- **WHEN** the user commits a rename to `Cloud agent options`

- **THEN** the exploration's list label is `Cloud agent options`

- **AND** a subsequent load of explorations still shows `Cloud agent options` for that
  exploration

> test: code

### Scenario: Blank rename leaves the label unchanged

- **GIVEN** an exploration whose display name is `Cloud agent options`
- **WHEN** the user commits a rename that is empty or only whitespace
- **THEN** the exploration's list label remains `Cloud agent options`

> test: code

## Requirement: Refresh retitles from the active session chat

When the user requests a title refresh for an exploration that has an active session with
at least one non-priming, non-bare-slash user message, the system SHALL run a title
summary against a representation of that session's current conversation (not limited to
the first such user message when later user turns exist), SHALL set the session's title to
the non-empty result even if a title was already set, and SHALL set the exploration's
display name to the same result. When no suitable chat content exists, the session is
streaming, or the summary fails or returns empty, the existing session title and
exploration display name SHALL remain unchanged.

> test: code

### Scenario: Refresh overwrites an existing title and exploration label

- **GIVEN** an exploration with display name `Old title`

- **AND** its active session already has title `Old title`

- **AND** the session has non-priming user content that yields a title summary
  `New direction`

- **WHEN** the user requests a title refresh for that exploration

- **THEN** the session title is `New direction`

- **AND** the exploration list label is `New direction`

> test: code

### Scenario: Refresh input includes later user turns when present

- **GIVEN** an active exploration session with a first non-bare user message `Hello`
- **AND** a later non-bare user message `Focus on rename and retitle in the CHANGE list`
- **WHEN** the title refresh request is built for that session
- **THEN** the summarizer input includes the later user message, not only `Hello`

> test: code

### Scenario: Refresh with no summarizable content leaves labels unchanged

- **GIVEN** an exploration with display name `Keep me`
- **AND** its active session has no non-priming, non-bare-slash user message
- **WHEN** the user requests a title refresh
- **THEN** the exploration list label remains `Keep me`
- **AND** the session title is left unchanged

> test: code

### Scenario: Failed or empty refresh leaves labels unchanged

- **GIVEN** an exploration with display name `Keep me` and a titled active session
- **AND** a title refresh is requested
- **WHEN** the summary fails or returns only whitespace
- **THEN** the exploration list label remains `Keep me`
- **AND** the session title remains the prior non-empty title

> test: code
