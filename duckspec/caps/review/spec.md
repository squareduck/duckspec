# Change review record

A review is an advisory, document-schema artifact stored as a sequentially numbered file
in a change's `reviews/` directory — recognized and validated as a document, and appended
with the next number on creation.

A review is an advisory, document-schema artifact stored as a sequentially numbered file
in a change's `reviews/` directory — recognized and validated as a document, and appended
with the next number on creation. Creation is kind-aware: a review or followup shares the
same log, and the kind is encoded in the filename slug prefix.

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

Creating a review or a followup SHALL place it in the change's `reviews/` directory with a
two-digit number one greater than the highest existing critique file in that directory,
leaving existing files unchanged. Both kinds share one sequence. Creation SHALL be
rejected when a file whose full slug (including the kind prefix) already exists in the
change.

> test: code

### Scenario: The first review in a change is numbered 01

- **GIVEN** a change with no reviews
- **WHEN** a review is created
- **THEN** the new review file is numbered `01`
- **AND** its slug begins with `review-`

> test: code
> - crates/duckpond/src/plan.rs:729

### Scenario: A new review is numbered above the highest existing review

- **GIVEN** a change whose highest existing review is numbered `02`
- **WHEN** another review is created
- **THEN** the new review file is numbered `03`
- **AND** the existing reviews are left unchanged
- **AND** its slug begins with `review-`

> test: code
> - crates/duckpond/src/plan.rs:743

### Scenario: A review whose slug already exists is rejected

- **GIVEN** a change that already has a review whose full slug is `review-initial`
- **WHEN** a review whose title would produce that same full slug is created
- **THEN** creation is rejected

> test: code
> - crates/duckpond/src/plan.rs:763

### Scenario: A followup continues the shared sequence after a review

- **GIVEN** a change whose only critique file is numbered `01` with a `review-` slug
- **WHEN** a followup is created
- **THEN** the new file is numbered `02`
- **AND** its slug begins with `followup-`
- **AND** the existing review is left unchanged

> test: code
> - crates/duckpond/src/plan.rs:776

### Scenario: Review and followup with the same title portion both create

- **GIVEN** a change that already has a review whose full slug is `review-post-impl`

- **WHEN** a followup is created whose title slugifies to the same title portion
  `post-impl`

- **THEN** creation succeeds

- **AND** the new file's full slug is `followup-post-impl`

> test: code
> - crates/duckpond/src/plan.rs:796

## Requirement: Filename slug

Creating a review or followup SHALL derive a title slug from the human title by the
canonical slug rule, then form the filename slug by prefixing that title slug once with
`review-` or `followup-` respectively. Creation SHALL be rejected when the title yields an
empty title slug.

> test: code

### Scenario: A punctuated title produces a dash-normalized slug

- **GIVEN** a change and a review title containing punctuation, such as
  `Post-impl: soundness & fidelity`

- **WHEN** the review is created

- **THEN** the new review file's slug is `review-post-impl-soundness-fidelity`

> test: code
> - crates/duckpond/src/plan.rs:901

### Scenario: A title with no alphanumeric characters is rejected

- **GIVEN** a change and a review title with no alphanumeric characters
- **WHEN** the review is created
- **THEN** creation is rejected
- **AND** no review file is written

> test: code
> - crates/duckpond/src/plan.rs:919

### Scenario: A followup create prefixes the slug with followup-

- **GIVEN** a change and a followup title that slugifies to `collapse-policy`
- **WHEN** the followup is created
- **THEN** the new file's slug is `followup-collapse-policy`

> test: code
> - crates/duckpond/src/plan.rs:815
