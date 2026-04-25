# Step artifact parser

Parses a step artifact (`step-<slug>.md`) from a Layer 1 element stream into a typed
`Step` artifact: H1 title (slugified) + summary + named sections (Prerequisites / Context
/ Tasks / Outcomes) + checkbox-bearing items. Recognizes `@spec` and `@step` references;
enforces a non-empty Tasks section.

## Requirement: Step document structure

A step file SHALL consist of, in order: an H1 heading carrying the title, a summary
paragraph, an optional description, and a sequence of named H2 sections. Recognized
section names are `Prerequisites`, `Context`, `Tasks`, and `Outcomes`. The H1 title SHALL
be slugified (lowercased, non-alphanumerics collapsed to `-`) onto the `Step.slug` field
for filename matching. Any H2 heading whose name is not one of the four recognized names
SHALL be rejected.

> test: code

### Scenario: Minimal step parses successfully

- **GIVEN** a source containing an H1 title, a summary paragraph, and a Tasks section with
  at least one task

- **WHEN** the step is parsed

- **THEN** a `Step` artifact is produced

- **AND** the `slug` field carries the slugified title

### Scenario: Step with prerequisites parses successfully

- **GIVEN** a source containing a Prerequisites section followed by a Tasks section
- **WHEN** the step is parsed
- **THEN** the `prerequisites` field is populated with the parsed items
- **AND** the `tasks` field is populated

### Scenario: Step with context parses successfully

- **GIVEN** a source containing a Context section followed by a Tasks section
- **WHEN** the step is parsed
- **THEN** the `context` field carries the section's body elements

### Scenario: Unknown section heading raises UnknownStepSection

- **GIVEN** a source containing an H2 whose name is not one of `Prerequisites`, `Context`,
  `Tasks`, or `Outcomes`

- **WHEN** the step is parsed

- **THEN** parsing fails with `ParseError::UnknownStepSection`

## Requirement: Tasks section

The Tasks section SHALL be present and SHALL contain at least one task. Tasks are
checkbox-bearing list items at indent 0; each task MAY carry one level of subtasks at
deeper indent. A subtask indented beyond four spaces SHALL be rejected. Task content
beginning with `@spec <capability> <requirement>:
<scenario>` SHALL be parsed as
`TaskContent::SpecRef`; otherwise the content is `TaskContent::Freeform`. Leading numeric
prefixes (e.g. `1. `, `1.1 `, `12. `) on task text SHALL be stripped before content
parsing.

> test: code

### Scenario: Tasks containing @spec references parse as SpecRef content

- **GIVEN** a source whose Tasks section contains an item beginning with
  `@spec <capability-path> <requirement>: <scenario>`

- **WHEN** the step is parsed

- **THEN** the task's `content` is `TaskContent::SpecRef` with the capability,
  requirement, and scenario fields populated

### Scenario: Task checkboxes and numeric prefixes are recognized and stripped

- **GIVEN** a source whose Tasks section contains items with mixed checkbox states (`[x]`,
  `[X]`, `[ ]`) and leading numeric prefixes (`1. `, `1.1 `)

- **WHEN** the step is parsed

- **THEN** each task's `checked` field reflects its checkbox state

- **AND** each task's content is the text after the checkbox with the numeric prefix
  stripped

### Scenario: Missing or empty Tasks section raises the relevant error

- **GIVEN** a source missing the Tasks section entirely, and a separate source with a
  Tasks section containing no items

- **WHEN** each step is parsed

- **THEN** the missing-Tasks source fails with `ParseError::MissingTasksSection`

- **AND** the empty-Tasks source fails with `ParseError::EmptyTasksSection`

### Scenario: Subtask indented beyond four spaces raises SubtaskTooDeep

- **GIVEN** a source containing a subtask whose indent exceeds four spaces
- **WHEN** the step is parsed
- **THEN** parsing fails with `ParseError::SubtaskTooDeep`

## Requirement: Prerequisites section

Prerequisites items SHALL be checkbox-bearing list entries. An item whose text (after the
checkbox) begins with `@step ` SHALL be parsed as `PrerequisiteKind::StepRef` with the
trailing slug captured; otherwise the item is `PrerequisiteKind::Freeform` carrying the
raw text.

> test: code

### Scenario: Prerequisites with @step references parse as StepRef kind

- **GIVEN** a source whose Prerequisites section contains both `@step <slug>` items and
  freeform-text items, with mixed checkbox states

- **WHEN** the step is parsed

- **THEN** items beginning with `@step ` produce `PrerequisiteKind::StepRef` with the
  trailing slug captured

- **AND** other items produce `PrerequisiteKind::Freeform` with the raw text

- **AND** each item's `checked` field reflects its checkbox state
