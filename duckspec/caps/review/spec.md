# Change review record

A review is an advisory, document-schema artifact stored as a sequentially numbered file
in a change's `reviews/` directory — recognized and validated as a document, and appended
with the next number on creation.

## Requirement: Review recognition and validation

A markdown file under a change's `reviews/` directory SHALL be recognized as a review
artifact and validated against the document schema. Recognition SHALL hold for reviews in
both active and archived changes.

> test: code

### Scenario: A well-formed review validates

- **GIVEN** a review file under a change's `reviews/` directory with an H1 title and a
  summary

- **WHEN** the file is validated

- **THEN** validation succeeds

> test: code
> - crates/duckpond/tests/review.rs:11

### Scenario: A review missing its H1 title is reported as a document error

- **GIVEN** a review file under a change's `reviews/` directory with no H1 title
- **WHEN** the file is validated
- **THEN** validation fails with a document-schema error

> test: code
> - crates/duckpond/tests/review.rs:27

### Scenario: A review in an archived change is still recognized

- **GIVEN** a review file under an archived change's `reviews/` directory
- **WHEN** the file is classified
- **THEN** it is recognized as a review artifact

> test: code
> - crates/duckpond/tests/review.rs:43

## Requirement: Sequential numbering

Creating a review SHALL place it in the change's `reviews/` directory with a two-digit
number one greater than the highest existing review, leaving existing reviews unchanged.
Creation SHALL be rejected when a review with the same slug already exists in the change.

> test: code

### Scenario: The first review in a change is numbered 01

- **GIVEN** a change with no reviews
- **WHEN** a review is created
- **THEN** the new review file is numbered `01`

> test: code
> - crates/duckpond/src/plan.rs:689

### Scenario: A new review is numbered above the highest existing review

- **GIVEN** a change whose highest existing review is numbered `02`
- **WHEN** another review is created
- **THEN** the new review file is numbered `03`
- **AND** the existing reviews are left unchanged

> test: code
> - crates/duckpond/src/plan.rs:702

### Scenario: A review whose slug already exists is rejected

- **GIVEN** a change that already has a review with a given slug
- **WHEN** a review whose name produces that same slug is created
- **THEN** creation is rejected

> test: code
> - crates/duckpond/src/plan.rs:720

## Requirement: Filename slug

A review's filename slug SHALL be derived from its title by the canonical slug rule.
Creation SHALL be rejected when the title yields an empty slug.

> test: code

### Scenario: A punctuated title produces a dash-normalized slug

- **GIVEN** a change and a review title containing punctuation, such as
  `Post-impl: soundness & fidelity`

- **WHEN** the review is created

- **THEN** the new review file's slug is `post-impl-soundness-fidelity`

> test: code
> - crates/duckpond/src/plan.rs:794

### Scenario: A title with no alphanumeric characters is rejected

- **GIVEN** a change and a review title with no alphanumeric characters
- **WHEN** the review is created
- **THEN** creation is rejected
- **AND** no review file is written

> test: code
> - crates/duckpond/src/plan.rs:812
