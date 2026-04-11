# @ Authentication

## @ Requirement: Email-password login

### + Scenario: Login with 2FA enabled

- **GIVEN** the user has two-factor authentication enabled
- **WHEN** the user submits correct credentials
- **THEN** the system issues an interim token
- **AND** redirects to the TOTP verification page
