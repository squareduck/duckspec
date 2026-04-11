# Implement session expiration

Add server-side session timeout logic and cover the scenarios with
tests.

## Context

The session middleware currently does not track last-access time.
This step adds that tracking and the expiration check.

## Tasks

- [ ] 1. Add `last_accessed_at` column to the `sessions` table
- [ ] 2. Update session middleware to refresh `last_accessed_at` on
      each request
- [ ] 3. @spec auth Session expiration: Idle timeout
- [ ] 4. @spec auth Session expiration: Force logout on password change
