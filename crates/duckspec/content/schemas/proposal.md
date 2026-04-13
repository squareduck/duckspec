# Proposal schema

A proposal describes **why** a change is needed and **what** it will
do at a high level. It is the pitch for the work — not the design,
not the spec, not the plan. Keep it short and persuasive.

## Structure

```markdown
# <Change Title>

<1-2 sentence summary>

## Motivation

<why this change, why now>

## Scope

### New capabilities
- `<capability-path>` — <one-line description>

### Modified capabilities
- `<capability-path>` — <what changes and why>

### Out of scope
- <what this change deliberately does NOT touch>

## Impact

<affected code, APIs, dependencies, breaking changes>
```

## Rules

- H1 title is required.
- A summary paragraph directly follows the H1.
- The body is freeform markdown — the sections above are recommended,
  not enforced by `ds check`.

## Quality

- **Motivation** answers "why" and "why now", not "what". If the
  motivation section reads like a feature list, it belongs in Scope.
- **Scope** is the contract between proposal and later stages. Name
  exact capability paths — the spec author works directly from this
  list. Be explicit about what is out of scope to prevent drift.
- **Impact** is for downstream effects: breaking changes, migration
  needs, affected teams or systems. Skip it if the change is
  self-contained.
- The entire proposal should fit on one screen. If it doesn't, the
  change is probably too big — split it.

## Example

```markdown
# Add Google OAuth login

Introduce Google as a third-party login option to reduce signup
friction for new users.

## Motivation

Analytics show 40% of signup drop-offs happen at password creation.
Google OAuth removes that friction for the largest user segment.

## Scope

  caps/
  ├── auth/
  │   ├── spec.md          (modified — session middleware fallback)
  │   └── google/           ← NEW
  │       └── spec.md       (OAuth 2.0 login flow)
  └── ...

### New capabilities
- `auth/google` — Google OAuth 2.0 login flow

### Modified capabilities
- `auth` — add fallback to OAuth identity in session middleware

### Out of scope
- Apple Sign In (deferred to a later change)
- Account linking UI for existing email-password users

## Impact

  ┌──────────┐    ┌──────────────┐    ┌─────────┐
  │ Login UI │───→│ Auth service  │───→│   DB    │
  └──────────┘    └──────────────┘    └─────────┘
       ↑ button       ↑ routes         ↑ table

- New `oauth_identities` table in the database
- New dependency on Google OAuth client library
- Login page UI gains a "Sign in with Google" button
```
