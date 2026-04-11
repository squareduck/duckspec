# @ Authentication

## @ Requirement: Email-password login

### + Scenario: Account lockout

- **GIVEN** three consecutive failed login attempts
- **WHEN** the user submits a fourth incorrect password
- **THEN** the account is locked for 15 minutes

> test: code

## + Requirement: Two-factor authentication

The system SHALL support TOTP-based 2FA for users who opt in.

> test: code

## = Requirement: Login

Email-password login

## - Requirement: Remember me
