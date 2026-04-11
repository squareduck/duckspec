# Add two-factor authentication

Introduce TOTP-based two-factor authentication to improve account
security for users who opt in.

## Why

Users with sensitive data have requested an additional layer of
security beyond email and password. TOTP is the most widely
supported second factor and does not require SMS infrastructure.

## What changes

- A new capability `auth/two-factor` with enrollment and
  verification specs.
- Modifications to the existing `auth` capability to gate login
  behind 2FA when enabled.
- Two implementation steps: library integration and enrollment flow.

## Out of scope

- Hardware security keys (WebAuthn) — deferred to a later change.
- Recovery codes — deferred pending UX design.
