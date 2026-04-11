# Add Google OAuth login

Introduce Google as a third-party login option to reduce signup
friction for new users.

## Why

Consumer users increasingly expect social login. Analytics show
roughly 40% of signup drop-offs happen at the password creation
step. Offering Google OAuth removes that friction.

## What changes

- A new capability `auth/oauth` with callback and token specs.
- UI: a "Sign in with Google" button on login and signup screens.
- Backend: OAuth 2.0 flow, token exchange, and user linking logic.
