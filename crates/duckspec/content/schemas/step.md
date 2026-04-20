# Step schema

A step is a **self-contained unit of implementation work**, sized to fit in a
single agent session. Steps are ordered, and each is processed in its own
`/ds-apply` invocation.

## Structure

```markdown
# <Human-readable step name>

<1-2 sentence summary>

## Prerequisites

- [ ] @step <other-step-slug>
- [ ] <freeform prerequisite>

## Context

<freeform prose: background the applying agent needs>

## Tasks

- [ ] 1. <task description>
  - [ ] 1.1 <subtask>
  - [ ] 1.2 <subtask>
- [ ] 2. <task description>
- [ ] 3. @spec <capability-path> <Requirement>: <Scenario>

## Outcomes

<populated during or after implementation>
```

## Rules

- Step files live at `changes/<name>/steps/NN-<slug>.md`.
- `NN` is a two-digit zero-padded number (step order).
- `<slug>` is the H1 title slugified to kebab-case.
- `## Tasks` is required with at least one task.
- `## Prerequisites`, `## Context`, `## Outcomes` are optional.
- Tasks use checkboxes with numeric prefixes (`1.`, `2.`, ...).
- Subtasks nest one level deep. Deeper nesting is invalid.
- A step is complete when all checkboxes are checked.
- The current step is the lowest-numbered step with unchecked tasks.

**Task content:**

- Freeform text describing work to do, or
- A single `@spec <capability-path> <Requirement>: <Scenario>` backlink — a
  scenario implementation task.

**Prerequisite content:**

- `@step <slug>` — reference to another step in the same change. Slug only — do
  **not** include the `NN-` filename prefix.
- Freeform text — any other precondition.

## Quality

- **Right-size steps.** Each step should be completable in one agent session. If
  a step has more than 7-8 tasks, it's probably too big.
- **Scenario tasks come from the spec.** Every `test: code` scenario in the
  change's specs must appear as an `@spec` task in some step. Don't leave
  scenarios uncovered.
- **Tasks are concrete.** "Implement X" not "Figure out X." If you need to
  figure something out, that's a Context paragraph or an unresolved open
  question in the design.
- **Order tasks by dependency.** Within a step, tasks should flow top-to-bottom:
  create the table before writing the query that uses it.
- **Prerequisites are informational.** The CLI doesn't enforce them, but the
  applying agent reads them to understand dependencies.
- **Context is the exception, not the rule.** Include a Context section only
  when the applying agent needs information the change's design and proposal
  don't provide — e.g., no design exists, or the design doesn't cover this
  step's implementation details. If the design already describes what this step
  implements, omit Context; don't duplicate.

## Example

```markdown
# Implement session expiration

Add server-side session timeout logic and cover the scenarios with integration
tests.

## Tasks

- [ ] 1. Add `last_accessed_at` column to the `sessions` table
- [ ] 2. Update session middleware to refresh `last_accessed_at` on each request
- [ ] 3. Add expiration check to `session_from_request()`
- [ ] 4. @spec auth Session expiration: Idle timeout
- [ ] 5. @spec auth Session expiration: Activity resets the timer
```
