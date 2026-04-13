# Doc delta schema

A doc delta describes **modifications** to an existing capability
doc. Uses the same marker system as spec deltas. Content under each
header is freeform (no GWT scenarios — this is a doc, not a spec).

## Structure

```markdown
# <marker> <Capability Title>

<optional new summary>

## <marker> <Section name>

<freeform content>
```

## Markers

Same markers as spec deltas:

| Marker | Name    | Operation                                        |
|--------|---------|--------------------------------------------------|
| `+`    | add     | Insert new section                               |
| `-`    | remove  | Delete section and subtree                       |
| `~`    | replace | Replace section content                          |
| `=`    | rename  | Rename section header                            |
| `@`    | anchor  | Optionally replace body, descend into children   |

## Rules

- Every H1, H2, and H3 must carry a marker.
- Same validation rules as spec deltas: canonical order, uniqueness,
  existence constraints.
- Content is freeform under each header — no GWT or test markers.

## Example

```markdown
# @ Authentication

## + Security rationale

Email-password was chosen over social-only login because consumer
users often distrust third-party identity providers.

## ~ Design decisions

- **30-minute timeout** — reduced from 60 minutes after security
  review. Balances security against user experience.
- **Server-side invalidation** — unchanged from original design.
```
