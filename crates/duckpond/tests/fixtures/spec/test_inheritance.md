# Email-password authentication

Users authenticate by submitting their email and password.

## Requirement: Email-password login

The system SHALL allow a registered user to authenticate using their
email address and password.

> test: code

### Scenario: Valid credentials

- **GIVEN** a user with a registered email and correct password
- **WHEN** the user submits the login form
- **THEN** the system issues a session token
- **AND** the user is redirected to their home page

### Scenario: Invalid password

- **GIVEN** a user with a registered email
- **WHEN** the user submits an incorrect password
- **THEN** the system rejects the login
- **AND** displays a generic "invalid credentials" error
- **AND** does not reveal whether the email is registered

### Scenario: Visual correctness of login button

- **GIVEN** the user is on the login page
- **WHEN** the page finishes loading
- **THEN** the sign-in button is visible and correctly styled

> manual: visual check during release review
