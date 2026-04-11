# Add two-factor authentication — Design

Implements TOTP-based 2FA using the existing session management
system, adding an enrollment flow and a verification step at login.

## Architecture

The 2FA check is inserted into the login flow as a post-password
step. After successful password verification, the system checks
whether the user has 2FA enabled. If so, it issues a short-lived
interim token and redirects to the TOTP input page. The session
token is only issued after successful TOTP verification.

## Data model

A new `totp_secrets` table stores encrypted TOTP secrets keyed by
user ID. The table has a `verified_at` timestamp that is null until
the user completes enrollment by entering a valid code.

## TOTP parameters

- Algorithm: SHA-1 (for maximum authenticator app compatibility)
- Digits: 6
- Period: 30 seconds
- Tolerance: 1 step in either direction
