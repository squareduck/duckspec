# Validated delta merge

The single entry point for applying a delta to an artifact and validating the result
against its schema. It returns the merged markdown together with the re-parsed artifact,
or a typed error that distinguishes a delta that failed to apply from merged text that
failed to re-parse.

## Requirement: Validated merge outcome

Merging a delta into an artifact SHALL apply the delta and re-parse the merged text with
the parser that matches the artifact kind — the spec parser for capability specs, the
document parser for docs. On success it SHALL return the rendered markdown together with
the parsed artifact. When the delta deletes the whole artifact (a remove marker on the
H1), it SHALL return a deletion outcome carrying no rendered text.

> test: code

### Scenario: A successful spec merge returns the rendered markdown and the parsed spec

- **GIVEN** a capability spec and a delta that applies cleanly to it
- **WHEN** the spec delta is merged
- **THEN** the result is an update carrying the rendered markdown
- **AND** the result carries the re-parsed spec

> test: code
> - crates/duckpond/tests/merge.rs:127

### Scenario: A delta that deletes the artifact yields a deletion outcome

- **GIVEN** a delta whose H1 carries the remove marker for the artifact's title
- **WHEN** the delta is merged
- **THEN** the result is a deletion outcome
- **AND** no rendered text is produced

> test: code
> - crates/duckpond/tests/merge.rs:162

### Scenario: A doc merge is validated with the document parser

- **GIVEN** a capability doc whose structure is a free heading tree with no `Requirement:`
  headings

- **AND** a delta that applies cleanly to it

- **WHEN** the doc delta is merged

- **THEN** the result is an update carrying the rendered markdown and the parsed document

> test: code
> - crates/duckpond/tests/merge.rs:173

## Requirement: Failure classification

A merge SHALL report failures in two distinct, observable categories: a delta that fails
to apply to its source SHALL return a merge error, and merged text that fails to re-parse
against its schema SHALL return a parse error. A failure carrying more than one underlying
error SHALL render as a single line — the first error's message followed by a count of the
remaining errors.

> test: code

### Scenario: A delta that does not apply returns a merge error

- **GIVEN** a delta that targets a heading absent from its source
- **WHEN** the delta is merged
- **THEN** the merge returns a merge error

> test: code
> - crates/duckpond/tests/merge.rs:196

### Scenario: Merged text that violates its schema returns a parse error

- **GIVEN** a delta that applies cleanly but whose merged result no longer satisfies the
  artifact's schema

- **WHEN** the delta is merged

- **THEN** the merge returns a parse error

> test: code
> - crates/duckpond/tests/merge.rs:218

### Scenario: A multi-error failure renders as one summarized line

- **GIVEN** a failure carrying several underlying errors

- **WHEN** the failure is rendered

- **THEN** the rendering is the first error's message followed by a count of the remaining
  errors

> test: code
> - crates/duckpond/tests/merge.rs:241
