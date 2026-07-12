# @ Harness selection

Model choices carry an explicit harness identity that persists across restarts, turns
dispatch to the provider their model names, and the default model cascade prefers a
per-chat pin, then a project override, then a global default — available only when that
preferred model is in the process catalog.

## @ Requirement: Default model resolution

Resolving the preferred model for a turn SHALL prefer a per-chat pin, then a project
override, then a global default. The preferred model is **available** only when it is
present in the process model catalog. When no preferred model is set at any cascade level,
or the preferred model is absent from the catalog, the model for the turn is not
available.

> test: code

### - Scenario: An empty cascade resolves to grok-4.5

### ~ Scenario: A per-chat pin overrides a project default

- **GIVEN** a per-chat pin
- **AND** a different project override
- **AND** a different global default
- **WHEN** the preferred model for a turn is resolved
- **THEN** the preferred model is the per-chat pin

> test: code

### + Scenario: A project override is preferred over the global default

- **GIVEN** no per-chat pin
- **AND** a project override
- **AND** a different global default
- **WHEN** the preferred model for a turn is resolved
- **THEN** the preferred model is the project override

> test: code

### + Scenario: The global default is preferred when pin and project override are unset

- **GIVEN** no per-chat pin
- **AND** no project override
- **AND** a global default
- **WHEN** the preferred model for a turn is resolved
- **THEN** the preferred model is the global default

> test: code

### + Scenario: A preferred model absent from the catalog is not available

- **GIVEN** a preferred model from the cascade
- **AND** that model absent from the process model catalog
- **WHEN** the effective model for a turn is resolved
- **THEN** the model for the turn is not available

> test: code

### + Scenario: With no preferred model at any cascade level, the model is not available

- **GIVEN** no per-chat pin
- **AND** no project override
- **AND** no global default
- **WHEN** the effective model for a turn is resolved
- **THEN** the model for the turn is not available

> test: code

## + Requirement: Global default model setting

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

### Scenario: An unset global default is seeded from the former built-in when that model is in the catalog

- **GIVEN** no configured global default
- **AND** a non-empty process model catalog that includes `grok` / `grok-4.5`
- **WHEN** the global default is seeded
- **THEN** the global default is `grok` / `grok-4.5`

> test: code

### Scenario: An unset global default is seeded from the first catalog model when the former built-in is absent

- **GIVEN** no configured global default
- **AND** a non-empty process model catalog that does not include `grok` / `grok-4.5`
- **WHEN** the global default is seeded
- **THEN** the global default is the first model in catalog order

> test: code

## + Requirement: Send requires an available model

A new main-chat turn SHALL NOT start when the effective model for the turn is not
available. The application SHALL NOT invent a substitute model in that case.

> test: code

### Scenario: A turn does not start when the preferred model is not available

- **GIVEN** an effective model that is not available
- **WHEN** the user attempts to send a main-chat turn
- **THEN** no new turn is started
- **AND** no substitute model is chosen for the turn

> test: code
