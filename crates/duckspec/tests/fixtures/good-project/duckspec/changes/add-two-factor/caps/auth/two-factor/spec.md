# Two-factor authentication

Provides TOTP-based second-factor verification for users who opt
in to enhanced account security.

## Requirement: TOTP enrollment

The system SHALL allow a user to enroll in TOTP-based 2FA by
generating a secret key and verifying it with a confirmation code.

> test: code

### Scenario: Successful enrollment

- **GIVEN** an authenticated user without 2FA enabled
- **WHEN** the user requests enrollment
- **THEN** the system generates a TOTP secret
- **AND** returns a QR code for the authenticator app

### Scenario: Confirm enrollment

- **GIVEN** a user with a pending TOTP secret
- **WHEN** the user submits a valid TOTP code
- **THEN** the secret is marked as verified
- **AND** future logins require TOTP verification

## Requirement: TOTP verification

The system SHALL require a valid TOTP code after password
verification for users with 2FA enabled.

> test: code

### Scenario: Valid TOTP code

- **GIVEN** a user who has completed password verification
- **AND** the user has 2FA enabled
- **WHEN** the user submits a valid TOTP code
- **THEN** the system issues a session token

### Scenario: Invalid TOTP code

- **GIVEN** a user who has completed password verification
- **WHEN** the user submits an invalid TOTP code
- **THEN** the system rejects the login with an error
- **AND** the interim token remains valid for retry
