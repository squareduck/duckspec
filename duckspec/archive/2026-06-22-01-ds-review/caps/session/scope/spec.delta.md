# @ Session scope orientation

## + Requirement: Current review in orientation

For a change scope, the orientation SHALL report the change's current review — the
highest-numbered review in the change — when the change has at least one review, and SHALL
omit any current-review report when the change has none. The presence or absence of
reviews SHALL NOT affect the change's reported progress or its suggested next stage.

> test: code

### Scenario: Orientation reports the highest-numbered review as the current review

- **GIVEN** a change scope whose change has more than one review
- **WHEN** the orientation is produced
- **THEN** it reports the highest-numbered review as the current review

> test: code

### Scenario: A change with no reviews reports no current review

- **GIVEN** a change scope whose change has no reviews
- **WHEN** the orientation is produced
- **THEN** it does not report a current review

> test: code

### Scenario: Adding a review does not change the suggested next stage

- **GIVEN** two change scopes with identical artifact and step state
- **AND** one of them additionally has reviews
- **WHEN** the orientation is produced for each
- **THEN** both report the same suggested next stage

> test: code
