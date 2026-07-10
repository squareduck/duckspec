# @ Change review record

A review is an advisory, document-schema artifact stored as a sequentially numbered file
in a change's `reviews/` directory — recognized and validated as a document, and appended
with the next number on creation. Creation is kind-aware: a review or followup shares the
same log, and the kind is encoded in the filename slug prefix.

## ~ Requirement: Sequential numbering

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

### Scenario: A new review is numbered above the highest existing review

- **GIVEN** a change whose highest existing review is numbered `02`
- **WHEN** another review is created
- **THEN** the new review file is numbered `03`
- **AND** the existing reviews are left unchanged
- **AND** its slug begins with `review-`

> test: code

### Scenario: A review whose slug already exists is rejected

- **GIVEN** a change that already has a review whose full slug is `review-initial`
- **WHEN** a review whose title would produce that same full slug is created
- **THEN** creation is rejected

> test: code

### Scenario: A followup continues the shared sequence after a review

- **GIVEN** a change whose only critique file is numbered `01` with a `review-` slug
- **WHEN** a followup is created
- **THEN** the new file is numbered `02`
- **AND** its slug begins with `followup-`
- **AND** the existing review is left unchanged

> test: code

### Scenario: Review and followup with the same title portion both create

- **GIVEN** a change that already has a review whose full slug is `review-post-impl`

- **WHEN** a followup is created whose title slugifies to the same title portion
  `post-impl`

- **THEN** creation succeeds

- **AND** the new file's full slug is `followup-post-impl`

> test: code

## ~ Requirement: Filename slug

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

### Scenario: A title with no alphanumeric characters is rejected

- **GIVEN** a change and a review title with no alphanumeric characters
- **WHEN** the review is created
- **THEN** creation is rejected
- **AND** no review file is written

> test: code

### Scenario: A followup create prefixes the slug with followup-

- **GIVEN** a change and a followup title that slugifies to `collapse-policy`
- **WHEN** the followup is created
- **THEN** the new file's slug is `followup-collapse-policy`

> test: code
