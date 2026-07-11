# Spec schema

A capability spec is the **behavior contract**: requirements and scenarios for
what the system must do. Scenarios marked `test: code` are a standing commitment
to automated verification.

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

## Quality

**Requirements**

- Use normative language precisely. SHALL means mandatory, SHOULD means
  recommended, MAY means optional. Do not write SHALL when you mean SHOULD.
- Each requirement covers one coherent behavioral concern. If its scenarios
  test unrelated things, split it.
- Normative prose stands on its own - scenarios illustrate, they do not replace
  the prose.
- Requirements describe **system behavior observers can care about**, not
  module placement, dependency graphs, or "does not call X." Those stay in
  `design.md` unless the public contract *is* a stability surface.

**Scenarios**

- **Falsifiability.** A scenario's THEN must be something a realistic-but-broken
  implementation could get wrong. If you cannot picture an implementation that
  would fail it - other than complete nonsense - the scenario is not encoding a
  contract; it is restating an identity. A getter returning what was set, a
  default value equaling the default, a method named toggle toggling - all pass
  "observable" but fail falsifiability. Drop them.
- **Outcome, not branch.** Scenarios are derived from the requirement, not the
  implementation. If you could not list them without first reading the code,
  they are mirroring it. A pure refactor that preserves observable behavior must
  not change the scenario list. Group by outcomes callers can observe; if two
  code paths converge on the same observable outcome, they are one scenario
  (parameterize GIVEN if the entry conditions differ).
- **Declarative, not procedural.** Describe what the system does, not how a user
  clicks through it. "WHEN the user submits the form" not "WHEN the user types
  their email, then tabs to password, then clicks submit."
- **GIVEN establishes state, not actions.** "GIVEN an authenticated user" not
  "GIVEN the user has logged in."
- **WHEN is a single trigger.** If you need multiple independent WHENs, you
  probably have two scenarios.
- **THEN is an observable outcome.** Not implementation details, internal state,
  private fields, enum variants, function names, or which branch ran. Restate in
  caller-observable terms - return value, side effect, persisted state, emitted
  event, response code. Prefer the **caller-meaningful** outcome; name storage
  only when durability or location *is* the contract. "THEN the session is
  invalidated" not "THEN the expire_session branch is taken" - and not "THEN the
  sessions table row is deleted" unless the table itself is the public contract.
- **Fewer, better.** Each scenario covers a distinct observable outcome, not a
  distinct code path. If two scenarios differ only trivially, merge them.
  Redundant scenarios are maintenance debt.
- **Every `test: code` is a commitment.** Only mark scenarios that genuinely need
  automated verification. Visual checks, deployment concerns, and
  documentation-only behaviors should use `manual:` or `skip:`.
- **Name scenarios by what is distinctive.** "Valid credentials" and "Invalid
  password" are good. "Test case 1" and "Happy path" are not.

**Two self-tests before committing a scenario list**

- **Refactor test.** If the implementation were rewritten - lookup table instead
  of if/else, polymorphism instead of switch, early returns instead of nesting -
  would these scenarios still describe the same behavior? If no, they are
  mirroring the code, not the contract.
- **Stranger test.** Could someone who has never seen the code write this
  scenario list from the requirement prose alone? If no, the scenarios are
  leaking the implementation.

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

### Scenario: Session expires after inactivity

- **GIVEN** an authenticated user
- **AND** their last request was more than 30 minutes ago
- **WHEN** the user makes a new request
- **THEN** the response is 401
- **AND** the session token is invalidated server-side

### Scenario: Activity resets the timer

- **GIVEN** an authenticated user
- **WHEN** the user makes a request at minute 29
- **THEN** the session remains valid for another 30 minutes
```
