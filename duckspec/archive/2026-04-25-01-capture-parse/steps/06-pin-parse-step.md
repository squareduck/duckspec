# Pin parse step

Wire `@spec` backlinks for the 9 `parse/step` scenarios. 3 are backed by existing
happy-path fixtures; 6 require new fixtures (3 errors plus 3 content-parsing fixtures
covering `@spec` refs, `@step` refs, and checkbox+numeric-prefix handling).

## Prerequisites

- [ ] @step tighten-parse-marked-heading

## Context

Create new fixture files and matching tests in `crates/duckpond/tests/parse_step.rs`. Use
`expect_err`+`assert_debug_snapshot` for error fixtures and the standard
`parse_fixture(...).expect("should parse")` pattern for new happy-path fixtures. Add
`// @spec parse/step <Requirement>: <Scenario>` comments above each test.

The 6 new fixtures:

```
tests/fixtures/step/
├── with_spec_refs.md                       ← TaskContent::SpecRef (happy)
├── with_step_refs.md                       ← PrerequisiteKind::StepRef (happy)
├── with_checkboxes_and_prefixes.md         ← checkbox + numeric prefix (happy)
└── errors/
    ├── unknown_section.md                  ← UnknownStepSection
    ├── tasks_missing_or_empty.md           ← MissingTasksSection + EmptyTasksSection
    └── subtask_too_deep.md                 ← SubtaskTooDeep
```

The combined `tasks_missing_or_empty.md` fixture should be split into two sub-fixtures
inside the same directory if a single source can't trigger both variants — a step file
with no Tasks section triggers `MissingTasksSection`, while one with `## Tasks` followed
by no list items triggers `EmptyTasksSection`. Use whichever shape gives the cleanest
snapshot.

## Tasks

- [x] 1. @spec parse/step Step document structure: Minimal step parses successfully

- [x] 2. @spec parse/step Step document structure: Step with prerequisites parses successfully

- [x] 3. @spec parse/step Step document structure: Step with context parses successfully

- [x] 4. @spec parse/step Step document structure: Unknown section heading raises UnknownStepSection

- [x] 5. @spec parse/step Tasks section: Tasks containing @spec references parse as SpecRef content

- [x] 6. @spec parse/step Tasks section: Task checkboxes and numeric prefixes are recognized and stripped

- [x] 7. @spec parse/step Tasks section: Missing or empty Tasks section raises the relevant error

- [x] 8. @spec parse/step Tasks section: Subtask indented beyond four spaces raises SubtaskTooDeep

- [x] 9. @spec parse/step Prerequisites section: Prerequisites with @step references parse as StepRef kind
