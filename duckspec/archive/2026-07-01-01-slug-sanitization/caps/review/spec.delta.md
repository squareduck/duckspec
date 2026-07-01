# @ Change review record

## + Requirement: Filename slug

A review's filename slug SHALL be derived from its title by the canonical slug rule.
Creation SHALL be rejected when the title yields an empty slug.

> test: code

### Scenario: A punctuated title produces a dash-normalized slug

- **GIVEN** a change and a review title containing punctuation, such as
  `Post-impl: soundness & fidelity`

- **WHEN** the review is created

- **THEN** the new review file's slug is `post-impl-soundness-fidelity`

> test: code

### Scenario: A title with no alphanumeric characters is rejected

- **GIVEN** a change and a review title with no alphanumeric characters
- **WHEN** the review is created
- **THEN** creation is rejected
- **AND** no review file is written

> test: code
