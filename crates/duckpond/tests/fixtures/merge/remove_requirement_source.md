# Authentication

Allows users to sign in with email and password.

## Requirement: Email-password login

The system SHALL allow a registered user to authenticate.

> test: code

### Scenario: Valid credentials

- **WHEN** the user submits correct credentials
- **THEN** the system issues a session token

## Requirement: Remember me

The system SHALL offer a "remember me" option.

> test: code

### Scenario: Checked remember me

- **WHEN** the user checks "remember me" during login
- **THEN** the session lasts for 30 days
