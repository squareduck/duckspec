# @ Authentication

## + Requirement: Two-factor authentication

The system SHALL support TOTP-based 2FA for users who opt in.

> test: code

### Scenario: 2FA enrollment

- **GIVEN** a user without 2FA enabled
- **WHEN** the user enables 2FA
- **THEN** a TOTP secret is generated
