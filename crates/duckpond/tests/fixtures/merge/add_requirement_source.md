# Authentication

Allows users to sign in with email and password.

## Requirement: Email-password login

The system SHALL allow a registered user to authenticate using their
email address and password.

> test: code

### Scenario: Valid credentials

- **GIVEN** a user with a registered email and correct password
- **WHEN** the user submits the login form
- **THEN** the system issues a session token
