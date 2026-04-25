# Wire up OAuth callback

Connect the Google OAuth callback handler into the existing session
middleware once the prerequisite steps land.

## Prerequisites

- [x] @step add-oauth-endpoints
- [ ] @step add-oauth-identities-table
- [ ] Google OAuth credentials are provisioned in staging

## Tasks

- [ ] 1. Implement `/auth/google/callback` handler
- [ ] 2. Exchange authorization code for tokens
