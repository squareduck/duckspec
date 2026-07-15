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

## Markers

| Marker | Name | Operation |
| --- | --- | --- |
| `+` | add | Insert new header and body |
| `-` | remove | Delete header and entire subtree |
| `~` | replace | Replace body and all children |
| `=` | rename | Rename header; preserve children |
| `@` | anchor | Optionally replace body; descend into children |

## Rules

- Path: `duckspec/changes/<name>/caps/<capability-path>/spec.delta.md`
- Every H1, H2, and H3 carries a marker; unmarked headers are invalid
- Marker is one character, then exactly one ASCII space, then heading text
- `+` is not valid on H1
- `+` targets a header that does not exist in the source
- `-`, `~`, `=`, `@` target headers that exist in the source
- `-` entries have an empty body
- `=` entries: only the new name on the first non-blank line after the header
- `@` is not valid on H3 (scenarios have no children - use `~`)
- Each header name appears at most once at a given level
- Canonical order within each level: `=` then `-` then `~` then `@` then `+`
  (parser sorts; author in any order)
- Children of `~` and `+` are content (no markers on nested headings under them
  as operations)
- Scenario and requirement bodies under `+` / `~` / `@` follow **spec** GWT and
  marker rules (`ds schema spec`)

## Quality

- **Lightest touch.** Prefer `@` + `+` children over rewriting whole requirements
- **Stable titles.** Prefer body-only `@` / `~` over rename (`=`) when the
  contract outcome is unchanged - renames break `@spec` backlinks
- **Rename then edit.** When a rename is required: `=`, then `@` (or `~`) under
  the **new** name for body changes
- **Merged result is a cold-reader spec.** Bodies are present-tense contract
  text, not change narration
- **New and rewritten bodies** satisfy `ds schema spec` Structure, Rules, and
  Quality. Do not `@`/`~` a requirement only to restate it in new words. Which
  scenarios to add is stage process (default omit) - not this schema
- Body markdown follows `style` (load only if not already in context)

## Formatting

After write or edit: `ds format <path>`. Presentation follows `style` - load only
if not already in context.

## Example

```markdown
# @ Authentication

## @ Requirement: Session expiration

### + Scenario: Force logout on password change

- **GIVEN** an authenticated user with an active session
- **WHEN** the user changes their password
- **THEN** all other sessions for that user are invalidated

> test: code

## + Requirement: Two-factor authentication

The system SHALL support TOTP-based 2FA for users who opt in.

> test: code

### + Scenario: 2FA enrollment

- **GIVEN** a user without 2FA enabled
- **WHEN** the user enables 2FA
- **THEN** a TOTP secret is generated
- **AND** a QR code is displayed for the authenticator app
```