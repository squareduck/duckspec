# Doc schema

A capability doc is the **human-readable counterpart** to its paired spec.
Readers go to the spec to learn exactly how a capability behaves; they go to
the doc to learn about the capability — what it is, how the pieces fit, how to
reason about it.

A well-formed doc describes the capability itself: its behavior, lifecycle,
states, modes, error handling, interactions with other capabilities, and
whatever else a reader needs to understand what the capability is. It reuses the
spec's vocabulary so a reader can cross-reference without translation.
Rationale, alternatives considered, and open questions belong in proposals and
codex entries — not here.

## Structure

```markdown
# <Capability Title>

<1-2 sentence summary>

<freeform markdown content>
```

## Rules

- H1 title is required and must match the paired spec's H1 exactly.
- A summary paragraph directly follows the H1.
- The body may contain any markdown: headers, prose, lists, code blocks, quotes,
  images, links.
- No structural validation beyond the H1 and summary.

## Quality

- **A minimal doc is for scaffolding, not for shipping.** H1 + summary is the
  structural minimum that satisfies pairing during early work. A shipped
  capability's doc covers the capability as a reader would need to understand
  it. Don't pad with content that restates the spec, but don't stop at the
  summary either.
- **Name H2s after what the capability actually has**, not after generic
  doc-template sections. Prefer `Session lifecycle`, `Token format`, `Retry
  behavior`, `Error handling`, `Concurrency`, `Rate limits` — whatever shape
  the capability actually has. Avoid generic sections like `Overview`, `Design
  decisions`, `Open questions`, `Rationale` — those either belong under the H1
  as prose or in a proposal or codex entry.
- **Tables and ASCII diagrams are tools, not decoration.** Use a table when
  listing parallel items with shared attributes (states, modes, error
  conditions, config options). Use an ASCII diagram when a flow, state machine,
  or structural relationship is genuinely easier to see than to read. When
  prose handles it, use prose. Both MUST be authored inside plain fenced code
  blocks — the formatter would otherwise reflow or corrupt them.
- **Docs describe current behavior, not change history.** Write the capability
  as it is *after* the change lands. Don't use phrases like "previously",
  "before the fix", or "now also supports" — those belong in PR descriptions and
  commit messages, not live documentation.

## Formatting

After writing or updating this artifact, run `ds format <path>` to apply
canonical formatting (line wrap, indentation, blank lines).

Use fenced code blocks for tables and diagrams; add a `<language>` tag to
fences that contain real code.

## Example

````markdown
# Authentication

Allows users to sign in with email and password. Primary auth mechanism for
consumer accounts. Sessions are opaque server-side tokens that expire on idle
and are invalidated on explicit sign-out.

## Session lifecycle

```
  sign-in ──▶ active ──idle 30m──▶ expired
                │                     │
              sign-out              sign-in
                │                     │
                ▼                     ▼
             revoked              (new session)
```

A session moves from `active` to `expired` after 30 minutes with no
authenticated request. Expired sessions cannot be reactivated — the user must
sign in again, which issues a new session.

## Error handling

Invalid credentials return a generic error regardless of which field was
wrong, to prevent user enumeration. Repeated failures from one IP are
throttled.

```
| Condition        | User-facing response     | Log tag           |
|------------------|--------------------------|-------------------|
| Unknown email    | "Invalid credentials"    | `auth.miss`       |
| Wrong password   | "Invalid credentials"    | `auth.miss`       |
| Unverified email | "Verify your email"      | `auth.unverified` |
| Throttled        | "Try again in N minutes" | `auth.throttle`   |
```

## Credentials

Passwords are stored as argon2id hashes with a per-user salt. The hash
parameters are fixed at write time; rotating them requires forcing a password
reset on the affected users.
````
