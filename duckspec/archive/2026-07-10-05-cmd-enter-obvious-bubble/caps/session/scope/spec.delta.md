# @ Session scope orientation

## @ Requirement: Lifecycle reflection

### - Scenario: A change with all steps complete reports completion and the review next-stage

### + Scenario: A change with all steps complete reports completion and the archive next-stage

- **GIVEN** a change scope whose steps are all complete
- **WHEN** the orientation is produced
- **THEN** it reports the steps as complete
- **AND** it suggests the archive stage as the next step

> test: code
