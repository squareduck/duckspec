# @ Authentication

## ~ Requirement: Session expiration

The system SHALL expire idle sessions after 15 minutes.

> test: code

### Scenario: Idle timeout

- **GIVEN** an authenticated user
- **WHEN** the user makes no requests for 15 minutes
- **THEN** the next request returns 401
