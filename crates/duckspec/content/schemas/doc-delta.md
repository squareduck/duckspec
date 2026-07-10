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

Deeper headings under a section follow the same marker rules when present.

## Markers

| Marker | Name | Operation |
| --- | --- | --- |
| `+` | add | Insert new section |
| `-` | remove | Delete section and subtree |
| `~` | replace | Replace section content |
| `=` | rename | Rename section header |
| `@` | anchor | Optionally replace body; descend into children |

## Rules

- Path: `duckspec/changes/<name>/caps/<capability-path>/doc.delta.md`
- Same marker mechanics as `ds schema spec-delta`: every heading marked; space
  after marker; no `+` on H1; existence rules for `+` vs `-`/`~`/`=`/`@`; empty
  body on `-`; rename name-only line on `=`; uniqueness; canonical order
  `=` `-` `~` `@` `+`
- Content under headers is freeform markdown - no GWT or test markers
- Bodies under `+` / `~` / `@` follow **doc** quality (`ds schema doc`)

## Quality

- **Cold reader.** Present-tense description of what is; no change narration
  ("previously", "now also"); no links into `changes/` or `archive/`
- **Keep pace with the spec.** Spec deltas that add modes, errors, or states
  usually need matching doc updates (tables, diagrams, prose)
- **Lightest touch.** Prefer `@` / `+` / targeted `~` over full-file rewrites
- Body markdown follows `style` (load only if not already in context)

## Formatting

After write or edit: `ds format <path>`. Presentation follows `style` - load only
if not already in context.

## Example

````markdown
# @ Authentication

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
