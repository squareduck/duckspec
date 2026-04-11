# Authentication

Allows users to sign in with email and password.

## Requirement: Session expiration

The system SHALL expire idle sessions after 30 minutes.

> test: code

### Scenario: Idle timeout

- **GIVEN** an authenticated user
- **WHEN** the user makes no requests for 30 minutes
- **THEN** the next request returns 401
