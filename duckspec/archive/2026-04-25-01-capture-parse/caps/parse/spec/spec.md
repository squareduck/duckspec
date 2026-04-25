# Capability spec parser

Parses a capability spec (`spec.md`) from a Layer 1 element stream into a typed `Spec`
artifact: H1 title, summary, optional description, and requirements with scenarios, GWT
clauses, and test markers.

This is the most-exercised Layer 2 parser in the system. It owns the shared L2 errors
(`ContentBeforeH1`, `MissingH1`, `MissingSummary`, `HeadingTooDeep`) on behalf of
`parse/doc`, `parse/delta`, and `parse/step`, which exercise the same code shape and rely
on this capability's coverage rather than re-asserting the errors locally.

## Requirement: Spec document structure

A spec SHALL consist of, in order: an H1 heading carrying the title, a single paragraph
carrying the summary, an optional description (any blocks before the first H2), and a
sequence of requirement sections introduced by H2 headings. Headings deeper than H3 SHALL
be rejected anywhere in the document. The parser SHALL accumulate errors rather than
short-circuit on the first problem.

> test: code

### Scenario: Minimal spec parses successfully

- **GIVEN** a source with an H1 title, a summary paragraph, and one requirement with one
  scenario

- **WHEN** the spec is parsed

- **THEN** a `Spec` artifact is produced

- **AND** the `requirements` list contains one entry

### Scenario: Spec with multi-paragraph description parses successfully

- **GIVEN** a source with multiple paragraphs between the summary and the first H2
- **WHEN** the spec is parsed
- **THEN** the description elements are captured on the `Spec` artifact in source order

### Scenario: Content before H1 raises ContentBeforeH1

- **GIVEN** a source whose first element is not an H1 heading
- **WHEN** the spec is parsed
- **THEN** parsing fails with `ParseError::ContentBeforeH1`

### Scenario: Missing H1 raises MissingH1

- **GIVEN** an empty source
- **WHEN** the spec is parsed
- **THEN** parsing fails with `ParseError::MissingH1`

### Scenario: Missing summary raises MissingSummary

- **GIVEN** a source containing an H1 with no following paragraph
- **WHEN** the spec is parsed
- **THEN** parsing fails with `ParseError::MissingSummary`

### Scenario: Headings deeper than H3 raise HeadingTooDeep

- **GIVEN** a source containing an H4 heading anywhere in the document
- **WHEN** the spec is parsed
- **THEN** parsing fails with `ParseError::HeadingTooDeep`

## Requirement: Requirement section structure

Every H2 heading in a spec SHALL begin with the literal prefix `Requirement: ` followed by
a name. The name SHALL NOT contain a colon. A requirement SHALL contain at least one of:
normative prose, scenarios, or both — empty requirements are rejected.

> test: code

### Scenario: Spec with multiple requirements parses successfully

- **GIVEN** a source with three H2 requirement sections, each with one or more scenarios
- **WHEN** the spec is parsed
- **THEN** three `Requirement` entries are produced in source order
- **AND** each entry's scenarios appear in source order

### Scenario: H2 without 'Requirement: ' prefix raises InvalidRequirementPrefix

- **GIVEN** a source containing an H2 whose content does not begin with `Requirement: `
- **WHEN** the spec is parsed
- **THEN** parsing fails with `ParseError::InvalidRequirementPrefix`

### Scenario: Requirement name containing a colon raises RequirementNameColon

- **GIVEN** a source containing an H2 whose name (after the `Requirement: ` prefix)
  contains a colon

- **WHEN** the spec is parsed

- **THEN** parsing fails with `ParseError::RequirementNameColon`

### Scenario: Requirement with neither prose nor scenarios raises EmptyRequirement

- **GIVEN** a source containing a requirement with no prose and no scenarios
- **WHEN** the spec is parsed
- **THEN** parsing fails with `ParseError::EmptyRequirement`

## Requirement: Scenario section structure

Every H3 heading in a spec SHALL begin with the literal prefix `Scenario: ` followed by a
name. A scenario body SHALL consist of exactly one unordered list of GWT clauses,
optionally followed by a test marker blockquote — no other content is permitted.

> test: code

### Scenario: H3 without 'Scenario: ' prefix raises InvalidScenarioPrefix

- **GIVEN** a source containing an H3 whose content does not begin with `Scenario: `
- **WHEN** the spec is parsed
- **THEN** parsing fails with `ParseError::InvalidScenarioPrefix`

### Scenario: Scenario missing WHEN or THEN raises MissingWhen and MissingThen

- **GIVEN** a source containing one scenario with only a `GIVEN` clause and another with
  only a `GIVEN` and `WHEN` clause

- **WHEN** the spec is parsed

- **THEN** parsing fails with `ParseError::MissingWhen` for the first scenario

- **AND** parsing fails with `ParseError::MissingThen` for the second scenario

### Scenario: Non-GWT content inside scenario body raises UnexpectedScenarioContent

- **GIVEN** a source containing a scenario whose body includes a paragraph or other
  non-list-item content

- **WHEN** the spec is parsed

- **THEN** parsing fails with `ParseError::UnexpectedScenarioContent`

## Requirement: GWT phase machine

GWT clauses SHALL progress through phases `GIVEN → WHEN → THEN`. The `AND` keyword SHALL
continue whichever phase came immediately before it. A scenario SHALL contain at least one
`WHEN` and at least one `THEN`. Out-of-order clauses SHALL be rejected. Unrecognized
keywords on a list item SHALL be rejected.

> test: code

### Scenario: Out-of-order GWT clauses raise GwtClauseOutOfOrder

- **GIVEN** a source containing scenarios that exercise the three distinct out-of-order
  branches: `AND` with no preceding clause, `THEN` before any `WHEN`, and
  `GIVEN` after a later phase

- **WHEN** the spec is parsed

- **THEN** parsing fails with `ParseError::GwtClauseOutOfOrder` for each invalid
  transition

### Scenario: Unrecognized GWT keyword raises InvalidGwtKeyword

- **GIVEN** a source containing a list item beginning with a `**KEYWORD**` that is not
  `GIVEN`, `WHEN`, `THEN`, or `AND`

- **WHEN** the spec is parsed

- **THEN** parsing fails with `ParseError::InvalidGwtKeyword`

## Requirement: Test markers

Test markers SHALL appear as blockquote items at the end of a requirement or scenario
body. Recognized prefixes are `test: code` (optionally followed by a `> -` backlink list
with one path per line), `manual: <reason>`, and `skip: <reason>`. If a scenario has no
test marker, the parser SHALL inherit the marker from its parent requirement. A scenario
with no marker whose parent has no marker is rejected. A blockquote that does not match
any recognized prefix is rejected.

> test: code

### Scenario: Test marker inherits from requirement to scenario

- **GIVEN** a source containing a requirement with `> test: code` and two scenarios where
  one has its own marker and one does not

- **WHEN** the spec is parsed

- **THEN** the scenario without its own marker resolves to the requirement's marker

- **AND** the scenario with its own marker keeps its own value

### Scenario: Test code marker carries backlinks

- **GIVEN** a source containing `> test: code` followed by `> -` backlink lines
- **WHEN** the spec is parsed
- **THEN** the test marker's `Code` variant carries the backlink paths in source order

### Scenario: Unrecognized test marker prefix raises InvalidTestMarker

- **GIVEN** a source containing a blockquote whose content does not match `test: code`,
  `manual:`, or `skip:`

- **WHEN** the spec is parsed

- **THEN** parsing fails with `ParseError::InvalidTestMarker`

### Scenario: Scenario with no marker and requirement with no marker raises UnresolvedTestMarker

- **GIVEN** a source containing a scenario without a test marker whose parent requirement
  also has no test marker

- **WHEN** the spec is parsed

- **THEN** parsing fails with `ParseError::UnresolvedTestMarker`
