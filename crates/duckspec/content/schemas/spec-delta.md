# Spec delta schema

A spec delta describes **modifications** to an existing capability spec. Every
heading carries a marker for the operation on the source. Merged bodies must
still satisfy the full **spec** schema.

## Structure

```markdown
# <marker> <Capability Title>

<optional new summary>

## <marker> Requirement: <requirement name>

<optional body>

### <marker> Scenario: <scenario name>

<optional body>
```

Under `@`, nested headings carry operation markers. Under `+` / `~`, nested
headings are plain content (no markers) — e.g. `### Scenario: …`.

## Markers

| Marker | Name | Body | Nested headings | Effect on source |
| --- | --- | --- | --- | --- |
| `+` | add | new body | content (no markers) | insert header and body |
| `-` | remove | must be empty | none | delete header and entire subtree |
| `~` | replace | always replace | content (no markers) | replace body; wipe children; re-list full new subtree |
| `=` | rename | new-name line only | none | rename header; preserve children; later ops use the **new** name |
| `@` | anchor | empty keeps body; non-empty replaces body | operations (each marked) | surgical: only listed child ops apply |

## Rules

- Path: `duckspec/changes/<name>/caps/<capability-path>/spec.delta.md`
- Every H1, H2, and H3 that is an **operation entry** carries a marker; unmarked
  operation headers are invalid
- Marker is one character, then exactly one ASCII space, then heading text
- `+` is not valid on H1
- `+` targets a header that does not exist in the source
- `-`, `~`, `=`, `@` target headers that exist in the source
- `-` entries have an empty body
- `=` entries: only the new name on the first non-blank line after the header
  (e.g. `Requirement: New name`)
- `@` with an empty body preserves the source body; a non-empty body replaces
  only the body (children still change only via child ops)
- `@` is not valid on H3 (scenarios have no children — rewrite a scenario with
  `~` under an `@` parent requirement)
- Under `@`, every child heading is an operation (`+` / `-` / `~` / `=` as
  allowed); under `+` and `~`, nested headings are content and **must not**
  carry markers (e.g. under `## + Requirement: …` write `### Scenario: …`, not
  `### + Scenario: …`)
- Each header name appears at most once at a given level
- Canonical order within each level: `=` then `-` then `~` then `@` then `+`
  (parser sorts; author in any order)
- Scenario and requirement bodies under `+` / `~` / `@` follow **spec** GWT and
  test-marker rules (`ds schema spec`)

## Quality

- **Choose the lightest op that fits:**

```
| Situation | Prefer |
| --- | --- |
| Few children add, remove, or edit | `@` parent + child ops |
| Most of a requirement's scenarios rewritten | `~` requirement + full new body and scenarios as content |
| Norm prose only; scenarios stay | `@` with body text, no H3s |
| Rename needed | `=` then `@` or `~` under the **new** name |
```

- **Stable titles.** Prefer body-only `@` / `~` over rename (`=`) when the
  contract outcome is unchanged — renames break `@spec` backlinks
- **Merged result is a cold-reader spec.** Bodies are present-tense contract
  text, not change narration
- **New and rewritten bodies** satisfy `ds schema spec` Structure, Rules, and
  Quality. Do not `@`/`~` a requirement only to restate it in new words. Which
  scenarios to add is stage process (default omit) — not this schema
- Body markdown follows `style` (load only if not already in context)

## Formatting

After write or edit: `ds format <path>`. Presentation follows `style` - load only
if not already in context.

## Example

```markdown
# @ Authentication

## = Requirement: Email-password login

Requirement: Email-password authentication

## - Requirement: Remember me

## @ Requirement: Session expiration

### - Scenario: Idle timeout at 30 minutes

### + Scenario: Force logout on password change

- **GIVEN** an authenticated user with an active session
- **WHEN** the user changes their password
- **THEN** all other sessions for that user are invalidated

> test: code

## + Requirement: Two-factor authentication

The system SHALL support TOTP-based 2FA for users who opt in.

> test: code

### Scenario: 2FA enrollment

- **GIVEN** a user without 2FA enabled
- **WHEN** the user enables 2FA
- **THEN** a TOTP secret is generated
- **AND** a QR code is displayed for the authenticator app
```
