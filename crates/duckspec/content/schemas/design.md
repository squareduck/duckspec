# Design schema

A design is the **technical approach** for a change: architecture, components,
sketches, decisions, and downstream impact - so the approach can be judged before
specs and implementation. It realizes the proposal’s intent; it is not the
pitch and not the behavioral contract.

## Structure

```markdown
# <Change Title> - Design

<1-2 sentence summary>

## Approach

<strategy, architecture, data flow - diagrams when they help>

## <Component or area>

<role, connections, boundaries>

<code sketch: real language, types, signatures - not full bodies>

## Impact

<code, APIs, dependencies, migrations, breakage - omit if none>

## Decisions

- **<decision>** - <choice>. Alternatives: <…>.

## Risks

- **<risk>** → <mitigation>

## Open questions

- <unresolved items that still affect the approach>
```

Recommended sections, not enforced by `ds check` beyond H1 + summary. Component
H2s are named for real pieces of the solution (modules, layers, tables) - as
many as the design needs.

## Rules

- H1 title is required; conventional form is `<Change Title> - Design`
- A non-empty summary paragraph follows the H1 directly
- Body is freeform markdown; the Structure skeleton is the expected shape
- Path: `duckspec/changes/<change-name>/design.md`

## Quality

- **Approach** is the big picture: how pieces fit, data flow, boundaries.
  Diagrams when they communicate faster than prose.
- **Components** are the core: each H2 is one coherent piece. Prose plus
  signature-level sketches in the project’s real language, types, and paths -
  enough to accept or reject the shape, not a PR draft.
- **Impact** is downstream effect of the *approach* (deps, migrations, APIs,
  breakage). Absent when there is none - no empty section for show.
- **Decisions** record non-obvious choices and rejected alternatives.
- **Risks** as `risk → mitigation`. Absent when none.
- **Open questions** are honest unknowns that still matter; do not paper over
  them with false certainty.
- Stay technical: approach and shape, not a re-pitch of motivation, and not a
  stand-in for behavioral contracts.
- Body markdown follows `style` (load only if not already in context).

## Formatting

After write or edit: `ds format <path>`. Presentation follows `style` - load only
if not already in context.

## Example

````markdown
# Add Google OAuth login - Design

Google OAuth 2.0 beside email-password auth; session layer stays shared.

## Approach

```
Client ──redirect──→ Google ──auth code──→ Callback
                                              │
                                    look up / create user
                                              ▼
                                          Session
```

New entry point; converges with password auth at session creation. No change to
session storage or expiration.

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

## Impact

- New `oauth_identities` table
- Google OAuth client dependency
- Login UI: "Sign in with Google"

## Decisions

- **Session reuse** - keep opaque session tokens. Alternative: JWT sessions
  (rejected: complexity without benefit at our scale).

## Risks

- **Google outage** → password login remains; OAuth control shows degraded state.

## Open questions

- Show "Sign in with Google" on login, signup, or both?
````
