# Add TOTP library

Integrate a TOTP library and set up the database table for storing
encrypted secrets.

## Tasks

- [ ] Add `totp-rs` crate to dependencies
- [ ] Create `totp_secrets` migration with columns: user_id,
  encrypted_secret, verified_at
- [ ] Implement `TotpService` struct with `generate_secret()` and
  `verify_code()` methods
- [ ] @spec auth/two-factor/TOTP enrollment/Successful enrollment
