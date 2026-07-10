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

- Precise norms: SHALL mandatory, SHOULD recommended, MAY optional
- One coherent concern per requirement; split when scenarios drift apart
- Normative prose stands alone; scenarios illustrate, they do not replace it

**Scenarios**

- **Falsifiable.** THEN is something a realistic broken implementation could
  get wrong - not identity restatements (getter returns set value, etc.)
- **Outcome, not branch.** Derived from the requirement, not the code. Same
  observable outcome = one scenario (parameterize GIVEN if entry conditions
  differ). A pure refactor that preserves behavior must not force a scenario
  rewrite
- **Declarative.** What the system does, not a click-path script
- **GIVEN is state**, not prior actions ("authenticated user", not "user has
  logged in")
- **WHEN is one trigger**; multiple independent triggers usually mean multiple
  scenarios
- **THEN is observable** to a caller (response, side effect, persisted state,
  event) - not private fields, branches, or function names
- **Fewer, better.** Distinct outcomes, not every code path; `test: code` only
  when automated verification is worth the commitment
- **Names mark what is distinctive** - not "Happy path" or "Test 1"

Self-checks: would these scenarios still hold if the implementation were
rewritten? Could someone write them from the requirement prose alone?

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
