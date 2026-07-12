# Session scope orientation

The orientation duckspec hands a coding agent at the start of a session — identifying the
active scope, and for a change, the change it must act on by default, its progress, and
its suggested next stage — delivered reliably on the session's first turn.

## Requirement: Change identification and authority

For a change scope, the orientation SHALL name the change, state that change artifacts
live under the project-root path `duckspec/changes/{name}/`, and establish the change as
the default target for change-acting commands. The orientation SHALL instruct the agent to
act on the named change unless the user names a different one, so that an ambiguous
project state never makes the agent ask which change to act on.

> test: code

### Scenario: Orientation names the scoped change as the default command target

- **GIVEN** a session scoped to a change
- **WHEN** the orientation is produced
- **THEN** it names that change
- **AND** it states that change artifacts live under `duckspec/changes/{name}/`
- **AND** it states that change-acting commands target that change by default
- **AND** it directs disambiguation to the case where the user names a different change

> test: code
> - crates/duckboard/src/scope.rs:226

## Requirement: Lifecycle reflection

For a change scope, the orientation SHALL report the change's step progress and a
suggested next stage that matches the change's artifact state, step completion, and
whether the change has any reviews — the first option of the review-aware lifecycle ladder
(including arms that also list `/ds-review` and `/ds-followup`). When steps remain
unfinished it SHALL report the incomplete progress; when every step is complete it SHALL
report completion.

### Scenario: A change with unfinished steps reports remaining work and the apply next-stage

- **GIVEN** a change scope whose steps include at least one incomplete step
- **WHEN** the orientation is produced
- **THEN** it reports progress that is not yet complete
- **AND** it suggests the apply stage as the next step

> test: code
> - crates/duckboard/src/area/change.rs:2442

### Scenario: A change with only a proposal reports the design next-stage

- **GIVEN** a change scope that has a proposal but no design, specs, or steps
- **WHEN** the orientation is produced
- **THEN** it suggests the design stage as the next step

> test: code
> - crates/duckboard/src/area/change.rs:2492

### Scenario: A change with all steps complete reports completion and the archive next-stage

- **GIVEN** a change scope whose steps are all complete
- **AND** the change has no reviews
- **WHEN** the orientation is produced
- **THEN** it reports the steps as complete
- **AND** it suggests the archive stage as the next step

> test: code
> - crates/duckboard/src/area/change.rs:2460

### Scenario: All steps complete with a review suggests the step next-stage

- **GIVEN** a change scope whose steps are all complete
- **AND** the change has at least one review
- **WHEN** the orientation is produced
- **THEN** it suggests the step stage as the next step

> test: code
> - crates/duckboard/src/area/change.rs:2477

## Requirement: Non-change scope orientation

For exploration, capability-tree, and codex scopes, the orientation SHALL describe that
scope and SHALL NOT report change progress or a change next-stage.

> test: code

### Scenario: An exploration scope signals early-stage work with no change artifacts

- **GIVEN** a session scoped to an exploration
- **WHEN** the orientation is produced
- **THEN** it describes the scope as early-stage exploration
- **AND** it does not report change progress or a change next-stage

> test: code
> - crates/duckboard/src/scope.rs:285

### Scenario: A capability-tree scope carries no change facts

- **GIVEN** a session scoped to the capability tree
- **WHEN** the orientation is produced
- **THEN** it describes the capability-tree scope
- **AND** it points at `duckspec/caps/` and `duckspec/project.md`
- **AND** it does not report change progress or a change next-stage

> test: code
> - crates/duckboard/src/scope.rs:308

### Scenario: A codex scope points at the codex tree

- **GIVEN** a session scoped to the codex
- **WHEN** the orientation is produced
- **THEN** it describes the codex scope
- **AND** it points at `duckspec/codex/` and `duckspec/project.md`
- **AND** it does not report change progress or a change next-stage

> test: code
> - crates/duckboard/src/scope.rs:335

## Requirement: Reliable first-turn delivery

The orientation SHALL be carried in the first turn's message body. It SHALL be present
even when the project has no `AGENTS.md`. It SHALL NOT be repeated on subsequent turns of
the same session.

> test: code

### Scenario: The first turn's message body carries the scope orientation

- **GIVEN** a new session with no prior turns
- **WHEN** the first turn is dispatched
- **THEN** the orientation is part of the message body sent on that turn

> test: code
> - crates/duckboard/src/area/interaction.rs:988

### Scenario: Orientation is present when the project has no AGENTS.md

- **GIVEN** a new session in a project with no `AGENTS.md`
- **WHEN** the first turn is dispatched
- **THEN** the orientation is part of the message body sent on that turn

> test: code
> - crates/duckboard/src/area/interaction.rs:1005

### Scenario: A resumed session does not repeat the orientation

- **GIVEN** a session that has already had its first turn
- **WHEN** a subsequent turn is dispatched
- **THEN** the orientation is not included again

> test: code
> - crates/duckboard/src/area/interaction.rs:1025

## Requirement: Current review in orientation

For a change scope, the orientation SHALL report the change's current review — the
highest-numbered file under the change's `reviews/` directory (whether kind-prefixed as
review or followup, or a legacy unprefixed name) — as the project-root path
`duckspec/changes/{name}/reviews/{filename}` when the change has at least one review, and
SHALL omit any current-review report when the change has none. The presence of reviews
SHALL NOT change reported step progress (done and total counts). The suggested next stage
SHALL follow the review-aware lifecycle (same first option of that ladder), so a review
may change the suggested next stage relative to an otherwise identical change without
reviews.

### Scenario: Orientation reports the highest-numbered review as the current review

- **GIVEN** a change scope whose change has more than one review

- **WHEN** the orientation is produced

- **THEN** it reports the highest-numbered review as the current review at
  `duckspec/changes/{name}/reviews/{filename}`

> test: code
> - crates/duckboard/src/area/change.rs:2516

### Scenario: A change with no reviews reports no current review

- **GIVEN** a change scope whose change has no reviews
- **WHEN** the orientation is produced
- **THEN** it does not report a current review

> test: code
> - crates/duckboard/src/area/change.rs:2538

### Scenario: Adding a review does not change reported step progress

- **GIVEN** two change scopes with identical step completion state
- **AND** one of them additionally has reviews
- **WHEN** the orientation is produced for each
- **THEN** both report the same step progress (done and total)

> test: code
> - crates/duckboard/src/area/change.rs:2553
