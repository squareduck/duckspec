# Doc delta schema

A doc delta describes **modifications** to an existing capability doc. Uses the
same marker system as spec deltas. Content under each header is freeform (no GWT
scenarios — this is a doc, not a spec).

## Structure

```markdown
# <marker> <Capability Title>

<optional new summary>

## <marker> <Section name>

<freeform content>
```

## Markers

Same markers as spec deltas:

| Marker | Name    | Operation                                      |
| ------ | ------- | ---------------------------------------------- |
| `+`    | add     | Insert new section                             |
| `-`    | remove  | Delete section and subtree                     |
| `~`    | replace | Replace section content                        |
| `=`    | rename  | Rename section header                          |
| `@`    | anchor  | Optionally replace body, descend into children |

## Rules

- Every H1, H2, and H3 must carry a marker.
- Same validation rules as spec deltas: canonical order, uniqueness, existence
  constraints.
- Content is freeform under each header — no GWT or test markers.

## Quality

- **Deltas produce a doc for a cold reader.** The `+` and `~` bodies you write
  are merged into a doc whose readers have no knowledge of the change, the
  proposal, the design, or what the section looked like before. The proposal,
  design, and steps are ephemeral and archived away with the change — don't
  reference `proposal.md`, `design.md`, or anything else under `changes/` or
  `archive/`, since those paths won't exist alongside the merged doc. Don't
  narrate the transition either, with phrases like "previously", "used to", or
  "now also" — rewrite in present tense to describe what is.
- **Keep the doc's tables, diagrams, and prose in step with spec changes.** A
  spec delta that adds a new failure mode usually implies a doc delta that
  adds a row to an error table or a paragraph explaining the mode. A spec
  delta that renames a state usually implies touching the lifecycle diagram.
  Don't let the doc drift from the capability it documents.

## Formatting

After writing or updating this artifact, run `ds format <path>` to apply
canonical formatting (line wrap, indentation, blank lines).

Use fenced code blocks for tables and diagrams; add a `<language>` tag to
fences that contain real code.

## Example

````markdown
# @ Authentication

## ~ Error handling

Invalid credentials return a generic error regardless of which field was
wrong, to prevent user enumeration. Repeated failures from one IP are
throttled, and sustained failures across multiple IPs trigger a temporary
account lock.

```
| Condition        | User-facing response     | Log tag           |
|------------------|--------------------------|-------------------|
| Unknown email    | "Invalid credentials"    | `auth.miss`       |
| Wrong password   | "Invalid credentials"    | `auth.miss`       |
| Unverified email | "Verify your email"      | `auth.unverified` |
| Throttled        | "Try again in N minutes" | `auth.throttle`   |
| Account locked   | "Contact support"        | `auth.locked`     |
```

## + Remember me

Trusted devices may opt into a 30-day session via a "remember me" checkbox at
sign-in. The extended session binds to the device fingerprint and is revoked
if the fingerprint changes.
````
