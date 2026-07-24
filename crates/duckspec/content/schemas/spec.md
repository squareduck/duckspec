# Spec schema

A capability spec is concise technical documentation backed by tests: short
requirement umbrellas group independent scenario proofs of the important
behavior. Scenarios own the observable detail; requirement prose is not a
second inventory of those cases.

## Structure

```markdown
# <Capability Title>

<1-2 sentence summary>

## Requirement: <requirement name>

<short plain-language description: cohesion and shared invariants only>

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
- A requirement needs umbrella prose, at least one scenario, or both (not empty)
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

- **Complete, not exhaustive.** Requirements and scenarios together describe
  every important behavior the capability owns. Important means a stable
  observable rule whose violation materially changes correctness, safety, data,
  interoperability, or user experience - not every input, branch, or
  implementation detail. Completeness lives in that pairing; umbrella prose
  does not carry the full contract alone.
- **Cohesive whole.** Requirements form the shortest clear grouping of the
  capability's proofs. Merge overlap, remove stale or misplaced behavior, and
  reorganize existing content when that improves the complete file.
- **Short plain umbrellas.** Every requirement has a brief plain-language
  description of the high-level concern: why its scenarios belong together, and
  any single shared invariant they all rely on (named concepts, not tunable
  numbers). It never previews, enumerates, field-lists, method-lists, or
  paraphrases the scenarios beneath it. Prefer about one sentence. Do not grow
  the umbrella as scenario count grows.
- **No modal house style.** Do not use SHALL / MUST / SHOULD / MAY as the default
  voice. Write ordinary present-tense technical English. Concrete cases and
  distinct outcomes belong only in scenarios.
- **Named thresholds, not baked constants.** Prefer named concepts (`idle
  timeout`, `retry budget`) over literal values (`30 minutes`, `3 retries`)
  unless the specific number is a product-defining rule that must not drift with
  implementation tuning. Code and tests own default values; baking them into the
  contract causes needless churn when they change.
- **One concern per requirement.** Split unrelated behavior, but do not invent
  requirements merely to hold scenarios or to mirror API surfaces (write path,
  read path, default) of the same rule.
- **Independent proof, not assertion inventory.** Each scenario is a unit of
  independent proof - a contract slice that can fail for a different reason than
  its siblings. Prefer one scenario whose test walks the full relevant path
  (e.g. unset → enable → disable) over a scenario per check. Opposite inputs,
  true/false mirrors, and the same fact observed on two surfaces are one
  scenario, not many. Parameterize inputs only when they still prove one story.
- **Do not mint fluff.** Reject non-goals, negative architecture notes, storage
  column names, and UI chrome as requirements or scenarios unless they are the
  behavior under contract. "Not a feature flag", "docs say X", and setup
  preconditions are not scenarios. Out-of-scope notes do not belong in
  requirement prose.
- **Tests inform the contract.** Existing tests may reveal stable intentional
  behavior missing from the spec. Helper and implementation tests need not
  become scenarios; important behavioral tests should have a natural spec
  owner.
- **Lean GWT.** Use only the state needed to understand the action and only
  independently important observable outcomes. GIVEN is state, WHEN is the
  action under test, THEN is the result. A multi-assert test may cover several
  states under one scenario without restating each assert as GWT. Omit setup
  narration, modal verbs in clauses, and restatements of the requirement.
- **Observer-facing.** Returns, persisted state, events, responses, and visible
  recovery are contract material. Private fields, module placement, function
  names, and branches belong to implementation or design.
- **Distinctive names.** Name the independent proof (e.g. the control or failure
  mode), not a single assert. Avoid "Happy path", "Test 1", true/false twins,
  and sentence-length restatements.

Body markdown follows `style` (load only if not already in context).

## Formatting

After write or edit: `ds format <path>`. Presentation follows `style` - load only
if not already in context.

## Example

Two scenarios only because they are independent failure modes (expire vs reset),
not opposite steps of one toggle. A single control's lifecycle stays one
scenario with one multi-assert test. The requirement umbrella states the shared
rule once; scenarios own the cases. Thresholds are named concepts, not baked
constants — the idle duration lives in code unless the product freezes a value.

```markdown
# Session expiration

Sessions expire after inactivity to limit stolen-token blast radius.

## Requirement: Idle timeout

Authenticated sessions expire after the idle timeout, measured from the last
request.

> test: code

### Scenario: Idle session expires

- **GIVEN** an authenticated user
- **AND** the idle timeout has elapsed without activity
- **WHEN** the user makes a new request
- **THEN** the request is rejected as unauthenticated
- **AND** the session is invalidated

### Scenario: Activity resets the timer

- **GIVEN** an authenticated user
- **WHEN** the user makes a request before the idle timeout
- **THEN** the idle timer is reset
```
