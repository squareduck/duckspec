# Authentication

Allows users to sign in with email and password.

## Requirement: Email-password login

The system SHALL allow a registered user to authenticate using their
email address and password.

> test: code

### Scenario: Valid credentials

- **WHEN** the user submits correct credentials
- **THEN** the system issues a session token

### Scenario: Invalid password

- **WHEN** the user submits an incorrect password
- **THEN** the system rejects the login with a generic error

## Requirement: Session expiration

The system SHALL expire idle sessions after 30 minutes.

> test: code

### Scenario: Idle timeout

- **GIVEN** an authenticated user
- **WHEN** 30 minutes pass with no requests
- **THEN** the next request returns 401

## Requirement: Remember me

The system SHALL offer a "remember me" option.

> test: code

### Scenario: Checked remember me

- **WHEN** the user checks "remember me"
- **THEN** the session lasts 30 days
