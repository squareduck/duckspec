# Implement enrollment

Wire the enrollment UI and API endpoints using the TOTP service
from the previous step.

## Prerequisites

- [ ] @step add-totp-library

## Context

The enrollment flow has two phases: initiation (generate secret,
show QR code) and confirmation (verify user-submitted code). Both
endpoints sit behind authentication middleware.

## Tasks

- [ ] Implement `GET /account/2fa/setup` endpoint
- [ ] Implement `POST /account/2fa/confirm` endpoint
- [ ] @spec auth/two-factor/TOTP enrollment/Confirm enrollment
- [ ] Add enrollment toggle to account settings page

## Outcomes

- Users can enable 2FA from their account settings.
- The TOTP secret is stored only after successful confirmation.
