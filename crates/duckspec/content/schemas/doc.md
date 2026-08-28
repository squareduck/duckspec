# Doc schema

A capability doc is cohesive technical documentation for understanding the
system the paired spec contracts and tests. It explains the capability's mental
model, flows, states, relationships, policies, and failure behavior without
repeating the spec as a catalogue.

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
- Bodies authored under a delta and the merged result after apply still must
  satisfy this schema. Delta shape (markers, ops) is `ds schema doc-delta` —
  not restated here.

## Quality

- **Cohesive technical model.** Explain the capability as one system: how its
  pieces relate, what states and flows matter, and how important policies and
  failures fit together. A title and summary are only a scaffold.
- **Paired, not duplicated.** Share vocabulary, boundaries, and meaning with
  the spec, but do not paraphrase requirements and scenarios line by line. The
  spec owns the behavioral contract; the doc makes the complete system
  understandable. Neither file holds paint the other was forbidden to contract.
- **Meaning, not paint.** A purely visual change must not require a doc edit.
  Color, font, icon, padding, alpha, tint, contrast recipe, px chrome, and
  paint tokens are not the reader model. Presence, hit-testing, semantic roles,
  and interaction are. If a restyle, theme swap, or spacing tweak would force
  a doc edit, the sentence does not belong.
- **Domain H2s.** Name sections after what the capability actually has -
  `Session lifecycle`, `Token format`, `Retry behavior`, `Error handling`,
  `Concurrency`, `Rate limits` - whatever shape it has. Avoid generic shells
  (`Overview`, `Design decisions`, `Open questions`, `Rationale`). Those either
  belong under the H1 as prose or in a proposal or codex entry.
- **Same visual identity as chat.** Body presentation follows `style` (load only
  if not already in context): **scan table** for parallel short fields (states,
  modes, errors, config), **figure** for flow, state machine, or structure,
  **section map** only when multi-field blocks need room, prose when prose is
  enough. Plain fences for tables and figures so formatters do not reflow them.
  Do not invent a layout family that chat would not use for the same facts.
- **Cold reader.** Someone who walks up to this file with no knowledge of the
  change, proposal, design, or prior versions. Present tense. Do not reference
  `proposal.md`, `design.md`, or anything under `changes/` or `archive/` - those
  paths will not exist beside the merged doc. Do not narrate the change
  ("previously", "before the fix", "now also supports").
- **Code-grounded.** Names, flows, states, and seams agree with the implemented
  system and its tests without turning the doc into a file or symbol inventory.

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
                 ┌──────────┐  idle timeout  ┌──────────┐
                 │  active  │ ─────────────► │ expired  │
                 └────┬─────┘                └──────────┘
                      │
                      │ sign-out
                      ▼
                 ┌──────────┐
                 │ revoked  │
                 └──────────┘
```

A session moves from `active` to `expired` after the idle timeout without an
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
