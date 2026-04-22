# Spec delta schema

A spec delta describes **modifications** to an existing capability spec. Every
header carries a marker declaring what operation to perform on the source spec.

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

| Marker | Name    | Operation                                      |
| ------ | ------- | ---------------------------------------------- |
| `+`    | add     | Insert new header and body                     |
| `-`    | remove  | Delete header and entire subtree               |
| `~`    | replace | Replace body and all children                  |
| `=`    | rename  | Rename header, preserve children               |
| `@`    | anchor  | Optionally replace body, descend into children |

## Rules

- Every H1, H2, and H3 must carry a marker. Unmarked headers are invalid.
- `+` targets a header that does not exist in the source.
- `-`, `~`, `=`, `@` target headers that exist in the source.
- `-` entries must have an empty body.
- `=` entries contain only the new name on the first non-blank line after the
  header. No other content.
- `@` is not valid on H3 (scenarios have no children — use `~`).
- Each header name at a given level appears at most once.
- Entries appear in canonical order within each level: `=` → `-` → `~` → `@` →
  `+`.

## Quality

- Prefer deltas over full-file replacements. Deltas make the change visible —
  reviewers see exactly what moved.
- Use `@` (anchor) to add scenarios to an existing requirement without
  disturbing its prose or other scenarios.
- When renaming and modifying, use `=` for the rename and a separate `@` entry
  with the new name for the modification.

## Formatting

After writing or updating this artifact, run `ds format <path>` to apply
canonical formatting (line wrap, indentation, blank lines).

Use fenced code blocks for tables and diagrams; add a `<language>` tag to
fences that contain real code.

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
