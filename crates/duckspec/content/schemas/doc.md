# Doc schema

A capability doc provides **narrative context** for a capability: background,
design decisions, user journeys, rationale. Unlike specs, docs are freeform
after the required preamble.

The doc complements the spec. The spec says what the system does; the doc says
why, for whom, and with what trade-offs.

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

- A minimal doc (H1 + summary, no body) is valid and often sufficient. Don't pad
  docs with content that restates the spec.
- Good doc sections: Background, User journey, Design decisions, Open questions,
  Rationale. Use whichever apply.
- If a design decision affects only this capability, put it here. If it spans
  capabilities, put it in a codex entry.
- **Docs describe current behavior, not change history.** Write the capability
  as it is *after* the change lands. Don't use phrases like "previously",
  "before the fix", or "now also supports" — those belong in PR descriptions and
  commit messages, not live documentation.

## Example

```markdown
# Session expiration

Sessions expire after inactivity to limit the blast radius of stolen tokens.

## Design decisions

- **30-minute timeout** — short enough to limit exposure, long enough to avoid
  disrupting active users. Revisit if user complaints increase.
- **Server-side invalidation** — the session token is invalidated on the server,
  not just expired by TTL. This ensures immediate revocation on password change.
```
