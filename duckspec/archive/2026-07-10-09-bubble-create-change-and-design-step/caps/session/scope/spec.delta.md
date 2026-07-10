# @ Session scope orientation

## @ Requirement: Lifecycle reflection

For a change scope, the orientation SHALL report the change's step progress and a
suggested next stage that matches the change's artifact state, step completion, and
whether the change has any reviews — the same first lifecycle option used for obvious
chrome. When steps remain unfinished it SHALL report the incomplete progress; when every
step is complete it SHALL report completion.

> test: code

### ~ Scenario: A change with all steps complete reports completion and the archive next-stage

- **GIVEN** a change scope whose steps are all complete
- **AND** the change has no reviews
- **WHEN** the orientation is produced
- **THEN** it reports the steps as complete
- **AND** it suggests the archive stage as the next step

> test: code

### + Scenario: All steps complete with a review suggests the step next-stage

- **GIVEN** a change scope whose steps are all complete
- **AND** the change has at least one review
- **WHEN** the orientation is produced
- **THEN** it suggests the step stage as the next step

> test: code

## @ Requirement: Current review in orientation

For a change scope, the orientation SHALL report the change's current review — the
highest-numbered review in the change — as the project-root path
`duckspec/changes/{name}/reviews/{filename}` when the change has at least one review, and
SHALL omit any current-review report when the change has none. The presence of reviews
SHALL NOT change reported step progress (done and total counts). The suggested next stage
SHALL follow the review-aware lifecycle (same first option as obvious chrome), so a review
may change the suggested next stage relative to an otherwise identical change without
reviews.

> test: code

### - Scenario: Adding a review does not change the suggested next stage

### + Scenario: Adding a review does not change reported step progress

- **GIVEN** two change scopes with identical step completion state
- **AND** one of them additionally has reviews
- **WHEN** the orientation is produced for each
- **THEN** both report the same step progress (done and total)

> test: code
