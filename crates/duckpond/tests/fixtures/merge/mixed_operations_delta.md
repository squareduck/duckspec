# @ Authentication

## = Requirement: Email-password login

Requirement: Email-password authentication

## - Requirement: Remember me

## @ Requirement: Session expiration

### + Scenario: Force logout on password change

- **GIVEN** an authenticated user with an active session
- **WHEN** the user changes their password
- **THEN** all other sessions for that user are invalidated

> test: code

## + Requirement: Two-factor authentication

The system SHALL support TOTP-based 2FA for users who opt in.

> test: code

### Scenario: 2FA enrollment

- **GIVEN** a user without 2FA enabled
- **WHEN** the user enables 2FA
- **THEN** a TOTP secret is generated
