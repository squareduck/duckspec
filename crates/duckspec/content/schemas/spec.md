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

## Quality

**Requirements:**

- Use normative language precisely. SHALL means mandatory, SHOULD means
  recommended, MAY means optional. Don't write SHALL when you mean SHOULD.
- Each requirement covers one coherent behavioral concern. If a requirement has
  scenarios that test unrelated things, split it.
- Normative prose stands on its own — scenarios illustrate, they don't replace
  the prose.

**Scenarios:**

- **Declarative, not procedural.** Describe *what the system does*, not *how a
  user clicks through it*. "WHEN the user submits the form" not "WHEN the user
  types their email, then tabs to password, then clicks submit."
- **GIVEN establishes state**, not actions. "GIVEN an authenticated user" not
  "GIVEN the user has logged in."
- **WHEN is a single trigger.** If you need multiple WHENs, you probably have
  two scenarios.
- **THEN is an observable outcome.** Not implementation details. "THEN the
  session is invalidated" not "THEN the sessions table row is deleted."
- **Fewer, better scenarios.** Each scenario should cover a distinct behavioral
  path. If two scenarios differ only trivially, merge them. Redundant scenarios
  are maintenance debt.
- **Every `test: code` is a commitment.** Only mark scenarios that genuinely
  need automated verification. Visual checks, deployment concerns, and
  documentation-only behaviors should use `manual:` or `skip:`.
- **Name scenarios by what's distinctive.** "Valid credentials" and "Invalid
  password" are good. "Test case 1" and "Happy path" are not.

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
