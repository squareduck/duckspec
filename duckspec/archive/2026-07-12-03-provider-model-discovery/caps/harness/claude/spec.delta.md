# @ Claude harness

## @ Requirement: Oneshot preferred model

Title-summary and reply-suggestion oneshots on the Claude harness SHALL select the
preferred oneshot model for that harness when that model is among the models the agent
advertises. When the preferred model is not advertised, those oneshots SHALL select
another advertised model rather than failing. Main conversation turns SHALL NOT be
required to use this preferred oneshot model (session model selection is separate).

> test: code

### ~ Scenario: Preferred oneshot model is selected when advertised

- **GIVEN** the Claude agent advertising available models that include the preferred
  oneshot model for that harness among others

- **WHEN** the harness selects a model for a title-summary or reply-suggestion oneshot

- **THEN** it selects the preferred oneshot model

> test: code

## + Requirement: Model discovery

Listing Claude models on the host SHALL return the models the owned Claude agent
advertises on initialize, each tagged with the Claude harness. Each listed model SHALL
carry a human-readable display name. When the agent advertises a context window for a
model, that listing SHALL carry the same window; when it does not, the listing SHALL leave
the window unknown. When discovery cannot obtain an advertised set, listing SHALL return
an empty list without panicking.

> test: code

### Scenario: Listed models come from the agent advertise set

- **GIVEN** the owned Claude agent advertising a set of available models on initialize
- **WHEN** the harness lists models
- **THEN** the listed models are exactly that advertised set
- **AND** each listed model is tagged with the Claude harness

> test: code

### Scenario: Each listed model carries a display name

- **GIVEN** the owned Claude agent advertising models with display names
- **WHEN** the harness lists models
- **THEN** each listed model carries a non-empty display name

> test: code

### Scenario: A model with a known context window carries that window

- **GIVEN** the owned Claude agent advertising a model with a known context window
- **WHEN** the harness lists models
- **THEN** that listed model carries the same context window

> test: code

### Scenario: Discovery failure yields an empty host list without panic

- **GIVEN** an environment where Claude model discovery cannot obtain an advertised set
- **WHEN** the harness lists models
- **THEN** the model list is empty
- **AND** the listing completes without panicking

> test: code

## + Requirement: Agent model advertise

On initialize, the owned Claude ACP agent SHALL advertise its available models to the
host. When live discovery of Claude models succeeds, that advertise set SHALL be the live
catalog. When live discovery fails, the agent SHALL advertise a curated alias fallback set
rather than an empty advertise set.

> test: code

### Scenario: Successful live discovery advertises those models on initialize

- **GIVEN** live Claude model discovery succeeding with a non-empty catalog
- **WHEN** the agent completes initialize
- **THEN** the initialize result advertises that live catalog

> test: code

### Scenario: Failed live discovery advertises the curated alias fallback

- **GIVEN** live Claude model discovery failing
- **WHEN** the agent completes initialize
- **THEN** the initialize result advertises the curated alias fallback set
- **AND** the advertise set is non-empty

> test: code
