# Spec schema

A capability spec is concise technical documentation backed by tests: cohesive
normative requirements completely describe the important behavior, while a
minimal set of scenarios provides executable proof of its distinct outcomes.

## Structure

```markdown
# <Capability Title>

<1-2 sentence summary>

## Requirement: <requirement name>

<normative prose: SHALL / MUST / SHOULD / MAY>

> test: code

### Scenario: <scenario name>

- **GIVEN** <initial state or context>
- **AND** <more initial state - continues GIVEN>
- **WHEN** <trigger or action>
- **AND** <co-occurring trigger condition - continues WHEN>
- **THEN** <expected outcome>
- **AND** <additional outcome - continues THEN>

> test: code
```

`**AND**` is optional after any clause. A test marker may sit on the requirement
(inherited by scenarios) and/or on each scenario.

## Rules

- Path: `duckspec/caps/<capability-path>/spec.md` or, in a change,
  `duckspec/changes/<name>/caps/<capability-path>/spec.md`
- H1 title required; non-empty summary paragraph follows it directly
- Every H2 is `Requirement: <name>`; no other H2s
- Every H3 is `Scenario: <name>`; no other H3s
- No H4 or deeper
- Requirement names must not contain colons
- A requirement needs normative prose, at least one scenario, or both (not empty)
- Scenario body: exactly one unordered list of GWT bullets, optionally then a
  test-marker blockquote - nothing else
- At least one `**WHEN**` and one `**THEN**` per scenario
- Clause keywords: `**GIVEN**`, `**WHEN**`, `**THEN**`, `**AND**`. `**AND**`
  continues the immediately preceding GIVEN/WHEN/THEN. No required clause order
  beyond WHEN + THEN
- Every scenario resolves to a test marker - its own or inherited from the
  parent requirement
- Marker prefixes: `test: code`, `manual: <reason>`, `skip: <reason>`

**`test: code` backlinks** live in source, not in the spec body: a single
unbroken `@spec <capability-path> <Requirement>: <Scenario>` comment above the
test. `ds audit` resolves them; wrapped comments are invisible. `ds sync`
stamps resolved `path:line` onto markers under top-level `caps/` (bookkeeping -
do not hand-edit those paths).

**Deltas and merges.** Bodies authored under a delta and the merged result after
apply still must satisfy this schema. Delta shape (markers, ops) is
`ds schema spec-delta` — not restated here.

## Quality

- **Complete, not exhaustive.** Normative prose describes every important
  behavior the capability owns. Important means a stable observable rule whose
  violation materially changes correctness, safety, data, interoperability, or
  user experience - not every input, branch, or implementation detail.
- **Cohesive whole.** Requirements form the shortest clear contract for the
  capability. Merge overlap, remove stale or misplaced behavior, and reorganize
  existing content when that improves the complete file.
- **Normative precision.** SHALL / MUST / SHOULD / MAY mean what they say. Put
  the full policy in requirement prose; do not repeat it in every scenario.
- **One concern per requirement.** Split unrelated behavior, but do not invent
  requirements merely to hold scenarios.
- **Scenarios earn tests.** Each scenario pins a distinct important outcome,
  boundary, policy, state transition, compatibility promise, or integration
  seam. Variations with the same meaningful outcome belong in one parameterized
  test, not duplicate scenarios.
- **Tests inform the contract.** Existing tests may reveal stable intentional
  behavior missing from the spec. Helper and implementation tests need not
  become scenarios; important behavioral tests should have a natural spec
  owner.
- **Lean GWT.** Use only the state needed to understand the trigger and only
  independently important observable outcomes. GIVEN is state, WHEN is one
  trigger, and THEN is the result. Omit setup narration, SHALL in clauses, and
  restatements of the requirement.
- **Observer-facing.** Returns, persisted state, events, responses, and visible
  recovery are contract material. Private fields, module placement, function
  names, and branches belong to implementation or design.
- **Distinctive names.** Name the outcome that differentiates the scenario;
  avoid "Happy path", "Test 1", and sentence-length restatements.

Body markdown follows `style` (load only if not already in context).

## Formatting

After write or edit: `ds format <path>`. Presentation follows `style` - load only
if not already in context.

## Example

```markdown
# Session expiration

Sessions expire after inactivity to limit stolen-token blast radius.

## Requirement: Idle timeout

The system SHALL expire authenticated sessions after 30 minutes of inactivity,
measured from the last request (not from login time).

> test: code

### Scenario: Idle session expires

- **GIVEN** an authenticated user
- **AND** 30 minutes have passed without activity
- **WHEN** the user makes a new request
- **THEN** the request is rejected as unauthenticated
- **AND** the session is invalidated

### Scenario: Activity resets the timer

- **GIVEN** an authenticated user
- **WHEN** the user makes a request before the idle timeout
- **THEN** the session remains valid for another 30 minutes
```
