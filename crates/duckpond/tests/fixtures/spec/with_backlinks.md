# Session expiration

Sessions expire after a period of inactivity.

## Requirement: Idle timeout

The system SHALL expire authenticated sessions after 30 minutes.

> test: code

### Scenario: Idle user

- **WHEN** the user makes no requests for 30 minutes
- **THEN** the next request returns 401

> test: code
> - crates/auth/tests/login.rs:42
> - crates/auth/tests/integration.rs:117
