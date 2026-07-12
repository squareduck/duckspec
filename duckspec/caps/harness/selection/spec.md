# Harness selection

Model choices carry an explicit harness identity that persists across restarts, turns
dispatch to the provider their model names, and the default model cascade resolves to
grok-4.5.

Model choices carry an explicit harness identity that persists across restarts, turns
dispatch to the provider their model names, and the default model cascade prefers a
per-chat pin, then a project override, then a global default — available only when that
preferred model is in the process catalog.

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
> - crates/duckchat/src/provider.rs:197

### Scenario: A legacy bare model id loads as the Claude harness

- **GIVEN** a persisted value that records only a model id, with no harness
- **WHEN** it is loaded
- **THEN** the loaded choice names the Claude harness and that model

> test: code
> - crates/duckchat/src/provider.rs:208

## Requirement: Default model resolution

Resolving the preferred model for a turn SHALL prefer a per-chat pin, then a project
override, then a global default. The preferred model is **available** only when it is
present in the process model catalog. When no preferred model is set at any cascade level,
or the preferred model is absent from the catalog, the model for the turn is not
available.

> test: code

### Scenario: A per-chat pin overrides a project default

- **GIVEN** a per-chat pin
- **AND** a different project override
- **AND** a different global default
- **WHEN** the preferred model for a turn is resolved
- **THEN** the preferred model is the per-chat pin

> test: code
> - crates/duckboard/src/area/interaction.rs:992

### Scenario: A project override is preferred over the global default

- **GIVEN** no per-chat pin
- **AND** a project override
- **AND** a different global default
- **WHEN** the preferred model for a turn is resolved
- **THEN** the preferred model is the project override

> test: code
> - crates/duckboard/src/area/interaction.rs:1007

### Scenario: The global default is preferred when pin and project override are unset

- **GIVEN** no per-chat pin
- **AND** no project override
- **AND** a global default
- **WHEN** the preferred model for a turn is resolved
- **THEN** the preferred model is the global default

> test: code
> - crates/duckboard/src/area/interaction.rs:1021

### Scenario: A preferred model absent from the catalog is not available

- **GIVEN** a preferred model from the cascade
- **AND** that model absent from the process model catalog
- **WHEN** the effective model for a turn is resolved
- **THEN** the model for the turn is not available

> test: code
> - crates/duckboard/src/area/interaction.rs:1034

### Scenario: With no preferred model at any cascade level, the model is not available

- **GIVEN** no per-chat pin
- **AND** no project override
- **AND** no global default
- **WHEN** the effective model for a turn is resolved
- **THEN** the model for the turn is not available

> test: code
> - crates/duckboard/src/area/interaction.rs:1053

## Requirement: Harness dispatch

The provider that runs a turn SHALL be the one named by the turn model's harness. The set
of models offered for selection SHALL include the models of every registered harness.

> test: code

### Scenario: A model's harness selects the provider that runs its turn

- **GIVEN** a model naming a particular harness
- **WHEN** a turn for that model is dispatched
- **THEN** the provider that runs the turn is the one identified by that harness

> test: code
> - crates/duckboard/src/agent.rs:482

### Scenario: The offered models span every registered harness

- **GIVEN** more than one registered harness, each offering models
- **WHEN** the selectable models are listed
- **THEN** the list includes models from every registered harness

> test: code
> - crates/duckboard/src/agent.rs:502

## Requirement: Global default model setting

The application SHALL store a global main-chat default model as an application setting
(not scoped to a project). When the global default is unset and the process model catalog
is non-empty, the application SHALL seed the global default: prefer the former built-in
model (`grok` / `grok-4.5`) when that model is in the catalog; otherwise use the first
model in catalog order.

> test: code

### Scenario: A configured global default is stored as an application setting

- **GIVEN** a harness-tagged model choice for the global main-chat default
- **WHEN** the global default setting is saved
- **THEN** that choice is stored as a global application setting

> test: code
> - crates/duckboard/src/config.rs:340

### Scenario: An unset global default is seeded from the former built-in when that model is in the catalog

- **GIVEN** no configured global default
- **AND** a non-empty process model catalog that includes `grok` / `grok-4.5`
- **WHEN** the global default is seeded
- **THEN** the global default is `grok` / `grok-4.5`

> test: code
> - crates/duckboard/src/agent.rs:743

### Scenario: An unset global default is seeded from the first catalog model when the former built-in is absent

- **GIVEN** no configured global default
- **AND** a non-empty process model catalog that does not include `grok` / `grok-4.5`
- **WHEN** the global default is seeded
- **THEN** the global default is the first model in catalog order

> test: code
> - crates/duckboard/src/agent.rs:766

## Requirement: Send requires an available model

A new main-chat turn SHALL NOT start when the effective model for the turn is not
available. The application SHALL NOT invent a substitute model in that case.

> test: code

### Scenario: A turn does not start when the preferred model is not available

- **GIVEN** an effective model that is not available
- **WHEN** the user attempts to send a main-chat turn
- **THEN** no new turn is started
- **AND** no substitute model is chosen for the turn

> test: code
> - crates/duckboard/src/area/interaction.rs:1068
