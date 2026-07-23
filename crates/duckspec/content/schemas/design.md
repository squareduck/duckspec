# Design schema

A design is the settled technical direction for a change: the durable synthesis
of the collaborative design session that specs and implementation can rely on
without unresolved questions. Its body is freeform so the solution determines
the document shape.

## Structure

```markdown
# <Change Title> - Design

<compact technical summary>

<freeform body shaped around the technical design>
```

The body may be organized by components, flows, state ownership, lifecycle,
data model, or any other structure that makes this particular design easiest
to understand. Use prose, headings, tables, lists, ASCII diagrams, and
signature-level code sketches as the material requires.

## Rules

- H1 title is required; conventional form is `<Change Title> - Design`
- A non-empty summary paragraph follows the H1 directly
- Body after the summary is freeform markdown
- The design contains no unresolved questions
- Path: `duckspec/changes/<change-name>/design.md`

## Quality

- Preserve the complete technical picture that specs and implementation need:
  responsibilities, boundaries, connections, state and data flow, and
  lifecycle where they matter.
- Make dependencies between design areas and components easy to see.
- Record consequential decisions and rejected alternatives when their
  reasoning will matter later; omit ceremonial decision logs.
- Include impact, compatibility, migration, failure handling, and risk where
  the design actually has them, close to the affected part of the design.
- Use real project language, types, modules, and paths. Signature-level sketches
  may clarify a contract; full implementation bodies do not belong here.
- Resolve every design question before writing. Do not preserve uncertainty for
  specs or implementation to discover.
- Do not repeat the proposal or turn the design into a behavioral spec.
- Body markdown follows `style` (load only if not already in context).

## Formatting

After write or edit: `ds format <path>`. Presentation follows `style` - load only
if not already in context.

## Example

````markdown
# Add Google OAuth login - Design

Google OAuth 2.0 beside email-password auth; session layer stays shared.

## Flow

```
Client ──redirect──→ Google ──auth code──→ Callback
                                              │
                                    look up / create user
                                              ▼
                                          Session
```

The new entry point converges with password auth at session creation. Session
storage and expiration remain unchanged.

## OAuth identity storage

```rust
pub struct OAuthIdentity {
    pub user_id: UserId,
    pub provider: OAuthProvider,
    pub external_id: String,
    pub refresh_token_enc: Vec<u8>,
}
```

## OAuth flow endpoints

```rust
pub fn begin_oauth(provider: OAuthProvider) -> Result<RedirectUrl> { todo!() }
pub fn handle_callback(code: &str) -> Result<Session> { todo!() }
```

## Storage and compatibility

- Add an `oauth_identities` table keyed by provider and external identity.
- Encrypt refresh tokens with the existing application-secret mechanism.
- Existing users and password sessions require no migration.

## Failure behavior

- A Google outage disables only the OAuth entry point; password login remains.
- A callback with invalid state is rejected before token exchange.
- A provider identity already linked to another user returns a conflict instead
  of merging accounts.

## Settled choices

- Reuse opaque sessions rather than introducing JWT sessions.
- Show one "Sign in with Google" action on both login and signup surfaces; both
  enter the same flow.
- Require an authenticated account-linking flow before attaching Google to an
  existing password account.
````
