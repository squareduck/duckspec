# Authentication

Allows users to sign in with email and password.

## Background

Email-password was chosen over username-password to align with how
users think about identity.

## Design decisions

- **Session duration**: 30 minutes of inactivity.
- **Password storage**: argon2id with per-user salt.
