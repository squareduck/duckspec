# Step artifact parser

Parses a step artifact from a Layer 1 element stream into a typed `Step` artifact. Steps
describe one unit of implementation work within a change — prerequisites, context, the
work itself (Tasks), and the verifiable outcomes.

## Sections

A step file has four named sections, all H2. Order is conventional but not enforced —
section names are matched by literal text:

```
| Section       | Required | Body shape                          | Role                                  |
|---------------|----------|-------------------------------------|---------------------------------------|
| Prerequisites | optional | checkbox list of references / text  | what must be true before starting     |
| Context       | optional | freeform markdown blocks            | background the implementer needs      |
| Tasks         | required | checkbox list of work items         | the imperative work plan              |
| Outcomes      | optional | freeform markdown blocks            | what's verified at the end            |
```

The H1 title is also slugified (lowercased, non-alphanumerics collapsed to `-`, runs of
`-` deduplicated) and stored in `Step.slug` for matching against the file's name. The
slugify function is `step::slugify`.

## Tasks vs Prerequisites

Both are checkbox lists, but their item types differ:

```
Tasks                              Prerequisites
─────                              ─────────────
- TaskContent::SpecRef             - PrerequisiteKind::StepRef
  (@spec <cap> <req>: <scen>)        (@step <slug>)
- TaskContent::Freeform            - PrerequisiteKind::Freeform
  (raw text)                         (raw text)
```

Tasks reference scenarios (the work-to-do); prerequisites reference other steps (ordering
within a change) or carry freeform "must be true" statements.

## Checkbox states

Three forms are recognized at the start of a list-item's text:

```
| Form  | checked | Meaning                          |
|-------|---------|----------------------------------|
| [x]   | true    | done                             |
| [X]   | true    | done (uppercase variant)         |
| [ ]   | false   | not done                         |
| (none)| false   | implicitly not done              |
```

Items without a checkbox parse as unchecked rather than rejected, so half-authored steps
still load.

## Numeric prefix stripping

Authors often number tasks for readability:

```
- [ ] 1. Implement the parser
- [ ] 2. Add fixture tests
- [ ] 2.1 Cover happy paths
- [ ] 2.2 Cover error paths
```

The leading numeric prefix (`<digits>(.<digits>)* `) is stripped before content parsing,
so "Implement the parser" becomes the task's text rather than "1. Implement the parser."
The numbering is purely visual; it doesn't affect ordering or identity.

## Reference syntax

Two reference forms are recognized at the start of an item's stripped text:

```
@spec <capability-path> <requirement-name>: <scenario-name>
@step <step-slug>
```

`@spec` references resolve via the audit / sync subsystem to the targeted scenario in a
capability spec; `@step` references resolve to another step file in the same change.
Neither reference is validated at parse time — that's `ds audit`'s job.

## Subtask depth

Tasks may have one level of subtasks at indent > 0 ≤ 4. Indents beyond four spaces raise
`SubtaskTooDeep`. This is a soft rule rooted in readability: deeper nesting suggests a
step that should split, not a deeper outline.

## Error catalogue

Step-specific variants:

```
| Variant              | Triggered by                                              |
|----------------------|-----------------------------------------------------------|
| MissingTasksSection  | step has no `## Tasks` heading                            |
| EmptyTasksSection    | `## Tasks` is present but contains no list items          |
| UnknownStepSection   | an H2 name is not one of the four recognized sections     |
| SubtaskTooDeep       | a subtask is indented more than four spaces               |
```

Shared L2 errors (`ContentBeforeH1`, `MissingH1`, `MissingSummary`) are documented in
`parse/spec` and apply here unchanged.
