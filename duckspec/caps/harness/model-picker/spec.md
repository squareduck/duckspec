# Harness model picker

The model picker presents selectable models grouped by their owning harness, and the usage
meter measures context fill against the selected model's own context window.

## Requirement: Harness-grouped choices

The choices offered by the model picker SHALL present each selectable model under its
owning harness, so models from different harnesses are distinguishable rather than
flattened into one undifferentiated list.

> test: code

### Scenario: Choices present each model under its harness

- **GIVEN** selectable models drawn from more than one harness
- **WHEN** the picker choices are built
- **THEN** each model appears under its owning harness

> test: code
> - crates/duckboard/src/widget/agent_chat.rs:2210

## Requirement: Context fill from the active model's window

The usage meter SHALL measure context fill against the selected model's context window. A
selected model with no known context window SHALL show no fill rather than a fill computed
against a wrong or assumed window.

> test: code

### Scenario: Fill is measured against the selected model's window

- **GIVEN** a selected model with a known context window
- **AND** a count of tokens used in the session
- **WHEN** the usage meter is computed
- **THEN** the fill is the used tokens relative to that model's window

> test: code
> - crates/duckboard/src/widget/agent_chat.rs:2240

### Scenario: A model with no known window shows no fill

- **GIVEN** a selected model with no known context window
- **WHEN** the usage meter is computed
- **THEN** the meter shows no fill

> test: code
> - crates/duckboard/src/widget/agent_chat.rs:2252
