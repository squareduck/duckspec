# Google OAuth

Full-replace spec that conflicts with the delta (there is no delta
here, but we'll use this to test other errors).

## Requirement: OAuth callback

Handle the callback.

> test: code

### Scenario: Valid callback

- **WHEN** the callback arrives
- **THEN** the system creates a session
