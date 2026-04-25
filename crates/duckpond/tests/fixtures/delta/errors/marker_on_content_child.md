# @ Authentication

## + Requirement: Two-factor authentication

The system SHALL support TOTP-based 2FA.

### + Scenario: Enrollment

- **GIVEN** a user without 2FA
- **WHEN** the user enables it
- **THEN** a TOTP secret is generated
