# Chat oneshot models

Global per-harness oneshot model preferences: resolved against the process model catalog,
editable in Settings when agent input hints are on, and used for title and reply oneshots.

## Requirement: Global per-harness oneshot preference

The application SHALL store an optional preferred oneshot model id for each harness as a
global setting (not scoped to a project). A configured preference for one harness SHALL
NOT imply a preference for another harness.

> test: code

### Scenario: A configured oneshot model for a harness is stored globally

- **GIVEN** a preferred oneshot model id for a harness
- **WHEN** the oneshot model setting is saved
- **THEN** that preference is stored as a global application setting for that harness

> test: code

### Scenario: Preferences are keyed by harness not by project

- **GIVEN** a preferred oneshot model for a harness
- **AND** more than one project
- **WHEN** the oneshot model setting is read in either project
- **THEN** the same global preference for that harness is returned

> test: code

## Requirement: Oneshot model resolution

Resolving the oneshot model for a harness SHALL prefer a configured model id when that id
is present in the process model catalog for that harness. When no configured id is
present, or the configured id is not in the catalog, resolution SHALL fall back to a
string-match default among that harness’s catalog models, then to the first catalog model
for that harness when no default match applies.

> test: code

### Scenario: Configured model is used when it is in the catalog

- **GIVEN** a configured oneshot model id for a harness
- **AND** that id present in the process model catalog for that harness
- **WHEN** the oneshot model for the harness is resolved
- **THEN** the resolved model is the configured id

> test: code

### Scenario: Missing or unknown config falls back to string-match default then first catalog model

- **GIVEN** no configured oneshot model for a harness, or a configured id absent from that
  harness’s catalog

- **AND** a non-empty catalog slice for that harness

- **WHEN** the oneshot model for the harness is resolved

- **THEN** the resolved model is the string-match default when a catalog model matches

- **AND** otherwise the resolved model is the first catalog model for that harness

> test: code

## Requirement: Settings pickers when hints enabled

When agent input hints are enabled, Settings SHALL offer an oneshot model picker for each
harness that has at least one model in the process model catalog. When agent input hints
are disabled, Settings SHALL NOT show those oneshot model pickers.

> test: code

### Scenario: With agent input hints on, each harness with catalog models offers an oneshot model picker

- **GIVEN** agent input hints enabled
- **AND** at least one harness with a non-empty catalog slice
- **WHEN** the Chat settings section is shown
- **THEN** an oneshot model picker is offered for each harness that has catalog models

> test: code

### Scenario: With agent input hints off, oneshot model pickers are not shown

- **GIVEN** agent input hints disabled
- **AND** at least one harness with a non-empty catalog slice
- **WHEN** the Chat settings section is shown
- **THEN** no oneshot model picker is shown

> test: code

## Requirement: Oneshots use the resolved preference

Title-summary and reply-suggestion oneshots for a harness SHALL use that harness’s
resolved oneshot model as the preferred model for the oneshot path.

> test: code

### Scenario: Title and reply oneshots for a harness use that harness’s resolved oneshot model

- **GIVEN** a resolved oneshot model for a harness
- **WHEN** a title-summary or reply-suggestion oneshot runs on that harness
- **THEN** the oneshot path prefers that resolved model

> test: code
