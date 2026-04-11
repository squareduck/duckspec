# @ Authentication

## ~ Design decisions

- **Session duration**: 15 minutes of inactivity.
- **Password storage**: argon2id with per-user salt.
- **Error messages**: generic "invalid credentials" to prevent enumeration.
