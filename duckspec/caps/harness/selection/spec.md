# Harness selection

Model choices carry an explicit harness identity that persists across restarts, turns
dispatch to the provider their model names, and the default model cascade resolves to
grok-4.5.

## Requirement: Harness-tagged model identity

A persisted model choice SHALL record both its harness and its model id and SHALL
round-trip without loss. A legacy persisted value that records only a model id SHALL load
as the Claude harness.

> test: code

### Scenario: A model choice round-trips its harness and model

- **GIVEN** a model choice naming a harness and a model
- **WHEN** it is persisted and then loaded back
- **THEN** the loaded choice names the same harness and model

> test: code
> - crates/duckchat/src/provider.rs:167

### Scenario: A legacy bare model id loads as the Claude harness

- **GIVEN** a persisted value that records only a model id, with no harness
- **WHEN** it is loaded
- **THEN** the loaded choice names the Claude harness and that model

> test: code
> - crates/duckchat/src/provider.rs:178

## Requirement: Default model resolution

Resolving the model for a turn SHALL prefer a per-chat pin, then a project default, then a
built-in default. When neither a per-chat pin nor a project default is set, resolution
SHALL yield grok-4.5.

> test: code

### Scenario: An empty cascade resolves to grok-4.5

- **GIVEN** neither a per-chat pin nor a project default
- **WHEN** the model for a turn is resolved
- **THEN** the resolved model is grok-4.5 on the grok harness

> test: code
> - crates/duckboard/src/area/interaction.rs:604

### Scenario: A per-chat pin overrides a project default

- **GIVEN** a per-chat pin and a different project default
- **WHEN** the model for a turn is resolved
- **THEN** the resolved model is the per-chat pin

> test: code
> - crates/duckboard/src/area/interaction.rs:615

## Requirement: Harness dispatch

The provider that runs a turn SHALL be the one named by the turn model's harness. The set
of models offered for selection SHALL include the models of every registered harness.

> test: code

### Scenario: A model's harness selects the provider that runs its turn

- **GIVEN** a model naming a particular harness
- **WHEN** a turn for that model is dispatched
- **THEN** the provider that runs the turn is the one identified by that harness

> test: code
> - crates/duckboard/src/agent.rs:213

### Scenario: The offered models span every registered harness

- **GIVEN** more than one registered harness, each offering models
- **WHEN** the selectable models are listed
- **THEN** the list includes models from every registered harness

> test: code
> - crates/duckboard/src/agent.rs:230
