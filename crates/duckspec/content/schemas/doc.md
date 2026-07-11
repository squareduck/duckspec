# Doc schema

A capability doc is the **human-readable counterpart** to its paired spec: what
the capability is, how pieces fit, and how to reason about it. The spec is the
exact contract; the doc is orientation. Shared vocabulary with the spec; no
rationale, alternatives, or open questions (those belong in proposals or codex).

## Structure

```markdown
# <Capability Title>

<1-2 sentence summary>

<body>
```

Body is freeform markdown (headings, prose, lists, tables, diagrams, code).

## Rules

- Path: `duckspec/caps/<capability-path>/doc.md` or, in a change,
  `duckspec/changes/<name>/caps/<capability-path>/doc.md`
- H1 title required and **identical** to the paired spec's H1
- Non-empty summary paragraph follows the H1 directly
- No further structural rules on the body

## Quality

- **Scaffold vs ship.** H1 + summary is the structural minimum for pairing
  during early work - not a finished capability doc. A shipped doc covers what
  a reader needs to understand the capability. Do not pad by restating the
  spec line-for-line; do not stop at the summary when the capability has real
  shape (lifecycle, modes, errors, interactions, …).
- **Domain H2s.** Name sections after what the capability actually has -
  `Session lifecycle`, `Token format`, `Retry behavior`, `Error handling`,
  `Concurrency`, `Rate limits` - whatever shape it has. Avoid generic shells
  (`Overview`, `Design decisions`, `Open questions`, `Rationale`). Those either
  belong under the H1 as prose or in a proposal or codex entry.
- **Structure when it helps.** Table for parallel items with shared attributes
  (states, modes, errors, config). Diagram for flow, state machine, or structure
  that is easier to see than to read. Prose when prose is enough. Presentation
  (including plain fences for tables/diagrams) follows `style` - load only if
  not already in context.
- **Cold reader.** Someone who walks up to this file with no knowledge of the
  change, proposal, design, or prior versions. Present tense. Do not reference
  `proposal.md`, `design.md`, or anything under `changes/` or `archive/` - those
  paths will not exist beside the merged doc. Do not narrate the change
  ("previously", "before the fix", "now also supports").

## Formatting

After write or edit: `ds format <path>`. Presentation follows `style` - load only
if not already in context.

## Example

````markdown
# Authentication

Email-password sign-in for consumer accounts. Sessions are opaque server-side
tokens that expire on idle and invalidate on sign-out.

## Session lifecycle

```
                         sign-in
                            │
                            ▼
                 ┌──────────┐  idle 30m  ┌──────────┐
                 │  active  │ ─────────► │ expired  │
                 └────┬─────┘            └──────────┘
                      │
                      │ sign-out
                      ▼
                 ┌──────────┐
                 │ revoked  │
                 └──────────┘
```

A session moves from `active` to `expired` after 30 minutes without an
authenticated request. Expired and revoked sessions are not reactivated; a new
sign-in issues a new session.

## Error handling

```
| Condition        | User-facing response     | Log tag           |
| ---------------- | ------------------------ | ----------------- |
| Unknown email    | "Invalid credentials"    | `auth.miss`       |
| Wrong password   | "Invalid credentials"    | `auth.miss`       |
| Unverified email | "Verify your email"      | `auth.unverified` |
| Throttled        | "Try again in N minutes" | `auth.throttle`   |
```

Invalid credentials use one generic user-facing error (no field enumeration).
Repeated failures from one IP are throttled.

## Credentials

Passwords are argon2id hashes with a per-user salt. Hash parameters are fixed at
write time; rotating them requires a password reset for affected users.
````