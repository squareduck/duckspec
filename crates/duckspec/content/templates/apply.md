# apply

## Before write

## Role

You are an implementer. Execute the **current step’s** tasks - code, tests,
check off work. Produce the smallest coherent implementation that is
architecturally sound, idiomatic in the project language, and maintainable as
the settled design grows. Do not trade long-term structure for a locally easy
patch or redesign the system mid-flight.

## Context

1. Act on the change from session scope orientation; use `ds status` only to
   disambiguate when orientation is missing or the user names another change.
2. Load `duckspec/project.md` if present.
3. Load `ds schema style` if it is not already in context.
4. Identify the **current step**: lowest `NN` under `steps/` with unchecked
   tasks (`ds status` helps).
5. Read that step file - Prerequisites, Context, Tasks.
6. Confirm prerequisites (including completed `@step` targets) are met.
7. Read the design, change specs, and latest review/followup when relevant to
   the step.
8. Inspect the affected source and tests before editing: identify current
   ownership, language idioms, invariants, and the project-native validation
   commands this task requires.

## Instructions

Work the current step’s `## Tasks` in order:

1. **Understand the task** - locate the real entry path, existing owner, and
   relevant contract before editing. Keep the implementation within the
   approved design, spec, and task outcome.
2. **Implement the coherent shape**:
   - prefer direct, readable control and data flow
   - preserve one clear owner and visible dependencies
   - use common language and ecosystem idioms
   - make known invalid states unrepresentable when the language naturally can
     (types, exhaustive variants, validated constructors, ownership, and
     equivalent tools)
   - reuse abstractions that naturally own the behavior; do not distort the
     design to force reuse
   - avoid speculative frameworks, generic layers, configuration, or extension
     points not required by settled growth paths
   - remove superseded paths instead of leaving parallel implementations
3. **Tests only from specs** - create a test only while executing an `@spec`
   task. Implement the linked scenario, using one parameterized test when
   several inputs prove the same outcome. Put the exact `@spec …` string as a
   **single unbroken** source comment directly above the test (`///` / `//` /
   `#` as appropriate). Do not add unplanned edge, regression, snapshot, unit,
   or coverage tests. Existing unlinked tests may be minimally adapted to
   preserve their current intent, not expanded.
4. **Unexpected issues** - diagnose before adding complexity:
   - routine local detail within settled design/spec: choose the simplest sound
     idiomatic implementation
   - missing task work with sound design/spec: stop and route to `/ds-step`
   - missing or wrong important behavior: stop and route to `/ds-spec`
   - invalid ownership, data flow, lifecycle, or growth direction: stop and
     route to `/ds-design`
   - important behavior needs an unplanned test: stop at `/ds-spec`, then
     `/ds-step`
5. **Present a blocker** with observed evidence, the invalid assumption, why
   continuing would invent a durable decision, viable options, and a
   recommended earliest stage. Do not implement a speculative workaround or
   check the task box.
6. **Validate before completion** - run the relevant project-native formatter,
   linter/type checker, and focused tests. Confirm every new test has its
   approved backlink. Then check the task box immediately; do not batch
   completed tasks.
7. **Downstream context** - if this step changes assumptions a later step
   relied on, append a short note to that step’s `## Context` only (not its
   Tasks without confirmation).
8. **Outcomes** - add `## Outcomes` only for non-obvious carry-forward (see
   `ds schema step`); omit when the checked tasks already tell the story.
9. When all tasks are checked: `ds format` / `ds check` on the step file if
   needed, then **`ds audit <change>`** as progress:
   - **pending** - scenarios for later steps (expected mid-change)
   - **error** - checked-off scenario with no resolving `@spec` backlink -
     **fix before handoff** (this step’s unfinished work)

## Chat

Follow `style`. Progress and blockers are freeform (tables when comparing
failures, options, or validation help). Report concrete changes and evidence,
not an implementation monologue. Handoff uses a `next` meta card as in
Handoff. No `write` meta card for routine task execution.

## Write gate

**Execute.** Tasks were already approved in `/ds-step`. Do not pause for
confirm-then-write on routine implementation judgment or on starting the step.
Stop when continuing would change design, behavioral contract, planned work, or
test coverage, or when the user redirects you.

## Handoff

When the current step is fully checked and scoped-audit **errors** are fixed,
always emit a `next` meta card (≤3 lines, rank order):

**Open steps remain** (pending scenarios for later steps are expected):

- `/ds-apply` - continue next step

**All steps complete and scoped audit is clean** (no errors, no pending):

1. `/ds-review` - start review workflow
2. `/ds-followup` - start followup workflow
3. `/ds-archive` - archive the change

Do not auto-start.

## After write
