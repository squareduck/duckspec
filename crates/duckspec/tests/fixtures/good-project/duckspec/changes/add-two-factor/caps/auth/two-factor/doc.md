# Two-factor authentication

TOTP-based second-factor verification for enhanced account
security, available as an opt-in feature.

## User journey

A user navigates to account settings, clicks "Enable 2FA," scans
the QR code with their authenticator app, and enters the displayed
code to confirm enrollment. From that point on, every login requires
both a password and a TOTP code.

## Design decisions

- **TOTP over SMS**: SMS is vulnerable to SIM-swapping; TOTP is
  more secure and does not require telecom infrastructure.
- **Interim token**: Avoids re-prompting for the password if the
  TOTP attempt fails.
- **Tolerance window**: One step in either direction accommodates
  minor clock skew between the server and authenticator apps.
