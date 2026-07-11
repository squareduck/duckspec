# @ Exploration promotion

## + Requirement: Chat focus after bound promotion

When a newly-present change directory is promoted into from a binding, duckboard SHALL
restore keyboard focus to the chat input after that promotion so the user can continue
typing without re-selecting the input. Detecting a newly-present change directory with no
binding SHALL NOT force chat input focus as a result of that detection alone.

> test: code

### Scenario: Bound promotion restores chat input focus

- **GIVEN** an exploration whose agent created a change by name
- **AND** that change's directory is detected as newly present
- **WHEN** promotion is evaluated
- **THEN** the exploration is promoted into that change
- **AND** the chat input has keyboard focus

> test: code

### Scenario: Unbound new change does not force chat input focus

- **GIVEN** a change directory detected as newly present with no binding for its name
- **AND** the chat input does not have keyboard focus
- **WHEN** promotion is evaluated
- **THEN** no exploration is promoted into the change
- **AND** the chat input still does not have keyboard focus

> test: code
