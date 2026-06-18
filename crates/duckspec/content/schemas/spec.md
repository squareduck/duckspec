# Spec schema

A capability spec is the **behavior contract**: what the system must do,
expressed as requirements and scenarios. Specs are the most consequential
artifact in duckspec — every scenario marked `test: code` becomes a maintenance
commitment.

## Structure

```markdown
# <Capability Title>

<1-2 sentence summary>

## Requirement: <requirement name>

<normative prose: SHALL/MUST/SHOULD statements>

> test: code

### Scenario: <scenario name>

- **GIVEN** <initial state or context>
- **AND** <more initial state — continues GIVEN>
- **WHEN** <trigger or action>
- **AND** <co-occurring trigger condition — continues WHEN>
- **THEN** <expected outcome>
- **AND** <additional outcome — continues THEN>

> test: code
```

`**AND**` is optional after any clause. Use it wherever you need a continuation
bullet — not just after `**THEN**`.

## Rules

- H1 title is required.
- A summary paragraph directly follows the H1.
- All H2s must start with `Requirement: `. No other H2s allowed.
- All H3s must start with `Scenario: `. No other H3s allowed.
- No H4 or deeper headings anywhere.
- Requirement names must not contain colons.
- A requirement must have normative prose, at least one scenario, or both. Empty
  requirements are invalid.
- A scenario body is exactly one unordered list of GWT bullets, optionally
  followed by a test marker blockquote. Nothing else.
- Every scenario must have at least one `**WHEN**` and one `**THEN**`.
- Clause keywords: `**GIVEN**`, `**WHEN**`, `**THEN**`, `**AND**`. `**AND**`
  continues whichever of the three came immediately before it — `**AND**` after
  `**GIVEN**` adds state, after `**WHEN**` adds a trigger condition, after
  `**THEN**` adds an outcome. All four keywords are positionally equal; the
  schema imposes no order beyond "at least one WHEN and one THEN."
- Every scenario must resolve to a test marker — either its own or inherited
  from the parent requirement.
- Test marker prefixes: `test: code`, `manual: <reason>`, `skip: <reason>`.

## Test markers and backlinks

A scenario's test marker declares how it is verified:

- `test: code` — verified by an automated test. The test is the contract's
  enforcement; this marker is a standing commitment to keep one.
- `manual: <reason>` — verified by a human; the reason says how/why.
- `skip: <reason>` — intentionally unverified; the reason says why.

A `test: code` scenario is linked to its test by a **source backlink**: a
single-line `@spec <capability-path> <Requirement>: <Scenario>` comment placed
directly above the test, in the source language's comment syntax. The backlink
lives in the code, not the spec — `ds audit` resolves it by scanning source
files, and reports any `test: code` scenario that no backlink resolves to. Write
the comment as one unbroken line; a wrapped comment is invisible to the scan.

`ds sync` records the resolved `path:line` of each backlink into the scenario's
`test: code` marker in `caps/`, as living coverage documentation. This is
bookkeeping derived from the source comments — never hand-edit those paths, and
note that `ds sync` only touches `caps/`, so it has an effect only after a change
is archived (while a change is in flight, its scenarios still live in the change
folder).

## Quality

**Requirements:**

- Use normative language precisely. SHALL means mandatory, SHOULD means
  recommended, MAY means optional. Don't write SHALL when you mean SHOULD.
- Each requirement covers one coherent behavioral concern. If a requirement has
  scenarios that test unrelated things, split it.
- Normative prose stands on its own — scenarios illustrate, they don't replace
  the prose.

**Scenarios:**

- **Falsifiability.** A scenario's THEN must be something a realistic-but-broken
  implementation could get wrong. If you can't picture an implementation that
  would fail it — other than complete nonsense — the scenario isn't encoding a
  contract; it's restating an identity. A getter returning what was set, a
  default value equaling the default, a method named `toggle` toggling — all
  pass "observable" but fail falsifiability. Drop them.
- **Outcome, not branch.** Scenarios are derived from the requirement, not the
  implementation. If you couldn't list the scenarios without first reading the
  code, they're mirroring it. A pure refactor that preserves observable
  behavior must not change the scenario list. Group scenarios by the *outcomes*
  callers can observe; if two code paths converge on the same observable
  outcome, they're one scenario (parameterize GIVEN if the entry conditions
  differ).
- **Declarative, not procedural.** Describe *what the system does*, not *how a
  user clicks through it*. "WHEN the user submits the form" not "WHEN the user
  types their email, then tabs to password, then clicks submit."
- **GIVEN establishes state**, not actions. "GIVEN an authenticated user" not
  "GIVEN the user has logged in."
- **WHEN is a single trigger.** If you need multiple WHENs, you probably have
  two scenarios.
- **THEN is an observable outcome.** Not implementation details, internal
  state, private fields, enum variants, function names, or which branch ran.
  Restate in caller-observable terms — return value, side effect, persisted
  state, emitted event, response code. "THEN the session is invalidated" not
  "THEN the sessions table row is deleted" and not "THEN the `expire_session`
  branch is taken."
- **Fewer, better scenarios.** Each scenario should cover a distinct
  *observable outcome*, not a distinct code path. If two scenarios differ only
  trivially, merge them. Redundant scenarios are maintenance debt.
- **Every `test: code` is a commitment.** Only mark scenarios that genuinely
  need automated verification. Visual checks, deployment concerns, and
  documentation-only behaviors should use `manual:` or `skip:`.
- **Name scenarios by what's distinctive.** "Valid credentials" and "Invalid
  password" are good. "Test case 1" and "Happy path" are not.

Two self-tests before committing a scenario list:

- **Refactor test.** If the implementation were rewritten — lookup table
  instead of if/else, polymorphism instead of switch, early returns instead of
  nesting — would these scenarios still describe the same behavior? If no, the
  scenarios are mirroring the code, not the contract.
- **Stranger test.** Could someone who has never seen the code write this
  scenario list from the requirement prose alone? If no, the scenarios are
  leaking the implementation.

## Formatting

After writing or updating this artifact, run `ds format <path>` to apply
canonical formatting (line wrap, indentation, blank lines).

Use fenced code blocks for tables and diagrams; add a `<language>` tag to
fences that contain real code.

## Example

```markdown
# Session expiration

Sessions expire after a period of inactivity to reduce the blast radius of
stolen tokens.

## Requirement: Idle timeout

The system SHALL expire authenticated sessions after 30 minutes of inactivity.
The timeout is measured from the last request, not from login time.

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

The first scenario uses `**AND**` after both `**GIVEN**` (to add a second state
fact) and `**THEN**` (to add a second outcome). The second scenario omits
`**AND**` entirely — it is never required.
