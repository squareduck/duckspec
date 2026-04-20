# Design schema

A design document describes the **technical approach** for a change. It shows
the shape of the solution — architecture, components, code sketches — so the
user can evaluate the approach before committing to specs and implementation.

Not every change needs a design. Skip it when the approach is obvious from the
proposal and specs alone.

## Structure

```markdown
# <Change Title> — Design

<1-2 sentence summary>

## Approach

<technical strategy, architecture — ASCII diagrams encouraged>

## <Component name>

<what this component does, why it exists, how it connects>

<code sketch: real language, real types, signature depth — no bodies>

## <Component name>

...

## Decisions

- **<decision>** — <chosen approach>. Alternatives: <X, Y>.

## Risks

- **<risk>** → <mitigation>

## Open questions

- <unresolved items that may affect implementation>
```

## Rules

- H1 title is required and should match the change title with a ` — Design`
  suffix.
- A summary paragraph directly follows the H1.
- The body is freeform markdown — the sections above are recommended, not
  enforced by `ds check`.

## Quality

- **Approach** is the big picture: how the pieces fit together, what the data
  flow looks like, where the boundaries are. Use ASCII diagrams freely — they
  communicate architecture faster than prose.
- **Component sections** are the heart of the design. Each H2 covers one
  coherent piece of the change: a new module, a modified layer, a new table. Mix
  prose explanation with code sketches naturally.
- **Code sketches** use the project's actual language and types. Show real
  struct fields, real function signatures, real module paths. Omit function
  bodies, boilerplate, imports, and error handling details. The reader should
  see *where* things land and *how they connect* — enough to say "yes, that's
  the right shape" or "no, that return type is wrong." This is an architect's
  whiteboard, not a PR draft.
- **Component sections seed the step phase.** Each component roughly maps to one
  or more implementation steps. Write them as self-contained units that a
  step-writer can decompose into ordered work.
- **Decisions** record choices that aren't obvious. Include alternatives that
  were considered and why they were rejected. Future readers (and agents) need
  this context.
- **Risks** use the format `<risk> → <mitigation>`. Skip if none.
- **Open questions** capture anything unresolved. These should be resolved
  before stepping — if they aren't, the step-writer must flag them.

## Example

```markdown
# Add Google OAuth login — Design

Implements Google OAuth 2.0 as a new authentication path alongside the existing
email-password flow, reusing existing session management.

## Approach

┌──────────┐   redirect    ┌─────────┐   auth code    ┌──────────┐
│  Client  │─────────────→│ Google  │──────────────→│ Callback │
└──────────┘               └─────────┘                └──────────┘
                                                        │
                                              look up / create user
                                                        │
                                                        ▼
                                                   ┌─────────┐
                                                   │ Session │
                                                   └─────────┘

OAuth adds a new entry point but converges with email-password auth at the
session layer. No changes to session storage or expiration.

## OAuth identity storage

New table and struct to link external provider accounts to internal users. A
user may have multiple OAuth identities (one per provider) plus an optional
email-password credential.

┌───────────────────────────────────────────────────┐
│ oauth_identities                                  │
├───────────────┬───────────────────────────────────┤
│ user_id (FK)  │ provider │ external_id │ refresh  │
└───────────────┴───────────────────────────────────┘

pub struct OAuthIdentity {
    pub user_id: UserId,
    pub provider: OAuthProvider,
    pub external_id: String,
    pub refresh_token_enc: Vec<u8>,
}

## OAuth flow endpoints

Two new routes handle the redirect-and-callback dance. Session creation reuses
existing session middleware.

pub fn begin_oauth(provider: OAuthProvider) -> anyhow::Result<RedirectUrl> { todo!() }
pub fn handle_callback(code: &str) -> anyhow::Result<Session> { todo!() }

## Session middleware changes

session_from_request() gains a fallback path:

  credential lookup
       │
       ├── password credential found → existing path
       │
       └── no password credential
               │
               └── check oauth_identities → create session

No changes to session struct or expiration logic.

## Decisions

- **Session reuse** — reuse existing opaque session tokens rather than issuing
  JWTs. Alternatives: JWT-based sessions (rejected: adds complexity without
  benefit for our scale).

## Risks

- **Google API outage** → users can still log in via email-password; OAuth
  button shows degraded state.

## Open questions

- Should "Sign in with Google" appear on both login and signup pages, or only
  signup?
```
