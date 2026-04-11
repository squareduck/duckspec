# Google OAuth

Allows users to authenticate via their Google account as an
alternative to email-password login.

## Requirement: OAuth callback

The system SHALL handle the Google OAuth callback, exchange the
authorization code for tokens, and create or link the user account.

> test: code

### Scenario: Valid callback

- **GIVEN** the user authorized access on Google
- **WHEN** the callback arrives with a valid authorization code
- **THEN** the system exchanges the code for tokens
- **AND** a session is created for the user

### Scenario: User denies authorization

- **WHEN** the callback arrives with an error parameter
- **THEN** the system redirects to the login page with a flash message
