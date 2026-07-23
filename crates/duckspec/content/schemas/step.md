# Step schema

A step is a focused, self-contained implementation slice for a change: one
coherent outcome expressed as ordered tasks, with optional prerequisites,
context, and outcomes when they carry useful information forward.

## Structure

```markdown
# <Human-readable step name>

<1-2 sentence summary>

## Prerequisites

- [ ] @step <other-step-slug>
- [ ] <freeform prerequisite>

## Context

<background the implementer needs that is not already in design or proposal>

## Tasks

- [ ] 1. <task description>
  - [ ] 1.1 <subtask>
- [ ] 2. <task description>
- [ ] 3. @spec <capability-path> <Requirement>: <Scenario>

## Outcomes

<only when something non-obvious must carry forward - see Quality>
```

`## Tasks` is required. `## Prerequisites`, `## Context`, and `## Outcomes` are
present only when they have content.

## Rules

- Path: `duckspec/changes/<change-name>/steps/NN-<slug>.md`
- `NN` is two-digit zero-padded order; `<slug>` is the H1 slugified to kebab-case
- H1 title and a non-empty summary paragraph after it are required
- `## Tasks` is required and must contain at least one task
- Tasks are checkboxes with numeric prefixes (`1.`, `2.`, …)
- Subtasks nest at most one level (`1.1`, `1.2`, …); deeper nesting is invalid
- A step is complete when all of its checkboxes are checked

**Task body** is either:

- freeform work description, or
- a single `@spec <capability-path> <Requirement>: <Scenario>` on one unbroken
  line (scenario implementation task). Do not wrap `@spec` across lines -
  continuation becomes an orphan and the scenario reference is lost.

**Prerequisite body** is either:

- `@step <slug>` - another step in the same change; slug only (no `NN-` prefix);
  one unbroken line, or
- freeform text for any other precondition

## Quality

- **Focused slice.** One coherent implementation outcome, completable after its
  prerequisites. Split a step when its tasks can progress independently or
  become a grab bag of unrelated work.
- **Concrete tasks.** Actionable work (“add column X”), not research placeholders.
- **Dependency order.** Within a step, earlier tasks enable later ones.
- **Scenario coverage.** Every `test: code` scenario the change introduces should
  appear as an `@spec` task in some step.
- **Context sparingly.** Only when the implementer needs material not already in
  design or proposal - do not duplicate them.
- **Outcomes sparingly.** Only for something the next session or reader needs
  that is not obvious from code, design, or checked tasks (discovery, deviation,
  handoff fact). No “we did the tasks” summary.
- Body markdown follows `style` (load only if not already in context).

## Formatting

After write or edit: `ds format <path>`. Presentation follows `style` - load only
if not already in context. `ds format` preserves long single-line `@spec` tasks;
it does not rejoin a reference already broken across lines.

## Example

```markdown
# Implement session expiration

Add server-side session timeout and cover it with integration tests.

## Tasks

- [ ] 1. Add `last_accessed_at` to the `sessions` table
- [ ] 2. Refresh `last_accessed_at` on each authenticated request
- [ ] 3. Enforce idle expiration in `session_from_request()`
- [ ] 4. @spec auth Session expiration: Idle timeout
- [ ] 5. @spec auth Session expiration: Activity resets the timer
```
