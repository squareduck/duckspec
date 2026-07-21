# Doc delta schema

A doc delta describes **modifications** to an existing capability doc. Same
marker system as **spec delta**; bodies are freeform (no GWT). Merged result
must still satisfy the **doc** schema (including H1 match with the paired
spec after merge).

## Structure

```markdown
# <marker> <Capability Title>

<optional new summary>

## <marker> <Section name>

<freeform content>
```

Under `@`, nested headings carry operation markers. Under `+` / `~`, nested
headings are plain content (no markers). Deeper sections follow the same rules
when present.

## Markers

| Marker | Name | Body | Nested headings | Effect on source |
| --- | --- | --- | --- | --- |
| `+` | add | new body | content (no markers) | insert section |
| `-` | remove | must be empty | none | delete section and subtree |
| `~` | replace | always replace | content (no markers) | replace section body; wipe children; re-list full new subtree |
| `=` | rename | new-name line only | none | rename section; preserve children; later ops use the **new** name |
| `@` | anchor | empty keeps body; non-empty replaces body | operations (each marked) | surgical: only listed child ops apply |

## Rules

- Path: `duckspec/changes/<name>/caps/<capability-path>/doc.delta.md`
- Every H1, H2, and deeper heading that is an **operation entry** carries a
  marker; unmarked operation headers are invalid
- Marker is one character, then exactly one ASCII space, then heading text
- `+` is not valid on H1
- `+` targets a header that does not exist in the source
- `-`, `~`, `=`, `@` target headers that exist in the source
- `-` entries have an empty body
- `=` entries: only the new name on the first non-blank line after the header
- `@` with an empty body preserves the source body; a non-empty body replaces
  only the body (children still change only via child ops)
- Under `@`, every child heading is an operation; under `+` and `~`, nested
  headings are content and **must not** carry markers
- Each header name appears at most once at a given level
- Canonical order within each level: `=` then `-` then `~` then `@` then `+`
  (parser sorts; author in any order)
- Content under headers is freeform markdown — no GWT or test markers
- Bodies under `+` / `~` / `@` follow **doc** quality (`ds schema doc`)

## Quality

- **Bodies under `+` / `~` / `@`** follow full `ds schema doc` Quality (scaffold
  vs ship, domain H2s, cold reader, structure when it helps)
- **Cold reader.** Present-tense description of what is; no change narration
  ("previously", "now also"); no links into `changes/` or `archive/`
- **Keep pace with the spec.** Spec deltas that add modes, errors, or states
  usually need matching doc updates (tables, diagrams, prose)
- **Choose the lightest op that fits:**

```
| Situation | Prefer |
| --- | --- |
| Few child sections add, remove, or edit | `@` parent + child ops |
| Most of a section's content rewritten | `~` section + full new body (and children as content if any) |
| Section body only; children stay | `@` with body text, no child ops |
| Rename needed | `=` then `@` or `~` under the **new** name |
```

- Body markdown follows `style` (load only if not already in context)

## Formatting

After write or edit: `ds format <path>`. Presentation follows `style` - load only
if not already in context.

## Example

````markdown
# @ Authentication

## - Design decisions

## ~ Error handling

Invalid credentials return a generic error regardless of which field was wrong,
to prevent user enumeration. Repeated failures from one IP are throttled;
sustained failures across many IPs trigger a temporary account lock.

```
| Condition        | User-facing response     | Log tag           |
| ---------------- | ------------------------ | ----------------- |
| Unknown email    | "Invalid credentials"    | `auth.miss`       |
| Wrong password   | "Invalid credentials"    | `auth.miss`       |
| Unverified email | "Verify your email"      | `auth.unverified` |
| Throttled        | "Try again in N minutes" | `auth.throttle`   |
| Account locked   | "Contact support"        | `auth.locked`     |
```

## + Remember me

Trusted devices may opt into a 30-day session via a "remember me" checkbox at
sign-in. The extended session binds to the device fingerprint and is revoked if
the fingerprint changes.
````
