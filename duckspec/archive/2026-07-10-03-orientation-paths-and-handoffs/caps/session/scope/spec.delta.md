# @ Session scope orientation

## @ Requirement: Change identification and authority

For a change scope, the orientation SHALL name the change, state that change artifacts
live under the project-root path `duckspec/changes/{name}/`, and establish the change as
the default target for change-acting commands. The orientation SHALL instruct the agent to
act on the named change unless the user names a different one, so that an ambiguous
project state never makes the agent ask which change to act on.

> test: code

### ~ Scenario: Orientation names the scoped change as the default command target

- **GIVEN** a session scoped to a change
- **WHEN** the orientation is produced
- **THEN** it names that change
- **AND** it states that change artifacts live under `duckspec/changes/{name}/`
- **AND** it states that change-acting commands target that change by default
- **AND** it directs disambiguation to the case where the user names a different change

> test: code

## @ Requirement: Lifecycle reflection

### = Scenario: A change with all steps complete reports completion and the archive next-stage

Scenario: A change with all steps complete reports completion and the review next-stage

### ~ Scenario: A change with all steps complete reports completion and the review next-stage

- **GIVEN** a change scope whose steps are all complete
- **WHEN** the orientation is produced
- **THEN** it reports the steps as complete
- **AND** it suggests the review stage as the next step

> test: code

## @ Requirement: Non-change scope orientation

### ~ Scenario: A capability-tree scope carries no change facts

- **GIVEN** a session scoped to the capability tree
- **WHEN** the orientation is produced
- **THEN** it describes the capability-tree scope
- **AND** it points at `duckspec/caps/` and `duckspec/project.md`
- **AND** it does not report change progress or a change next-stage

> test: code

### + Scenario: A codex scope points at the codex tree

- **GIVEN** a session scoped to the codex
- **WHEN** the orientation is produced
- **THEN** it describes the codex scope
- **AND** it points at `duckspec/codex/` and `duckspec/project.md`
- **AND** it does not report change progress or a change next-stage

> test: code

## @ Requirement: Current review in orientation

For a change scope, the orientation SHALL report the change's current review — the
highest-numbered review in the change — as the project-root path
`duckspec/changes/{name}/reviews/{filename}` when the change has at least one review, and
SHALL omit any current-review report when the change has none. The presence or absence of
reviews SHALL NOT affect the change's reported progress or its suggested next stage.

> test: code

### ~ Scenario: Orientation reports the highest-numbered review as the current review

- **GIVEN** a change scope whose change has more than one review

- **WHEN** the orientation is produced

- **THEN** it reports the highest-numbered review as the current review at
  `duckspec/changes/{name}/reviews/{filename}`

> test: code
