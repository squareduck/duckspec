# Harness model catalog

Process-local catalog of models from each available provider: refreshed once at app start,
kept across empty rediscovery failures, and used as the source for selectable models and
context-window lookup.

Process-local catalog of models from each available provider: refreshed once at app start,
cleared for a harness when rediscovery is empty or fails, and used as the source for
selectable models and context-window lookup.

## Requirement: Startup catalog refresh

At app start the process SHALL refresh the model catalog for each available provider from
that provider’s discovery path. A successful refresh for a harness SHALL replace that
harness’s catalog slice with the discovered models.

> test: code

### Scenario: App start refreshes models for each available provider

- **GIVEN** more than one registered provider that can offer models

- **WHEN** the app starts and the model catalog is refreshed

- **THEN** each available provider’s discovery path is used to populate that harness’s
  catalog slice

> test: code
> - crates/duckboard/src/agent.rs:517

### Scenario: Successful refresh replaces that harness’s catalog slice

- **GIVEN** a harness with a prior catalog slice
- **AND** a successful rediscovery that yields a different non-empty model set
- **WHEN** the catalog is refreshed for that harness
- **THEN** the harness’s catalog slice is the newly discovered set

> test: code
> - crates/duckboard/src/agent.rs:536

## Requirement: Catalog is the selection source

The models offered for selection SHALL be the contents of the process model catalog.
Looking up the context window for a selected model SHALL use the catalog entry for that
model’s harness and id when present.

> test: code

### Scenario: Offered selectable models are the catalog contents

- **GIVEN** a process model catalog with models from one or more harnesses
- **WHEN** the selectable models are listed
- **THEN** the listed models are exactly the catalog contents

> test: code
> - crates/duckboard/src/agent.rs:590

### Scenario: Context window lookup uses the catalog entry for the selected model

- **GIVEN** a catalog entry for a model with a known context window
- **AND** that model selected
- **WHEN** the context window for the selected model is resolved
- **THEN** the resolved window is the window from that catalog entry

> test: code
> - crates/duckboard/src/agent.rs:613

## Requirement: Clear slice on empty rediscovery

When rediscovery for a harness yields an empty set or fails, the catalog SHALL clear that
harness’s catalog slice (including any previous non-empty list). When a harness has never
had a successful discovery, an empty or failed rediscovery SHALL leave that harness’s
slice empty without panicking.

> test: code

### Scenario: Empty rediscovery clears the prior harness list

- **GIVEN** a harness whose catalog slice is non-empty
- **AND** a rediscovery for that harness that yields an empty set
- **WHEN** the catalog is refreshed for that harness
- **THEN** the harness’s catalog slice is empty

> test: code
> - crates/duckboard/src/agent.rs:560

### Scenario: Cold failure leaves that harness empty without panic

- **GIVEN** a harness with no prior successful discovery
- **AND** discovery for that harness failing or yielding an empty set
- **WHEN** the catalog is refreshed for that harness
- **THEN** the harness’s catalog slice is empty
- **AND** the refresh completes without panicking

> test: code
> - crates/duckboard/src/agent.rs:575
