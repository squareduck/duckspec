# Hook before/after + caps - Design

Hard-break hook create to `before`/`after`, keep template injection as-is, and backfill
`cli/hooks` with create, injection, and known-stages requirements — no shared path helper
and no stock-content ownership change.

## Components

```
duckpond plan                    duckspec CLI
────────────────                 ────────────────────────
HookPosition::{Before, After}    create hook --before|--after
as_str → "before" | "after"  ──► hooks/<stage>-{before|after}.md
STAGES (create-only gate)        skeleton: "# {Stage} - Before|After"

                                 template <stage>
                                 reads same path convention
                                 injects under ## Before/After write
```

```
| Piece | Responsibility |
| --- | --- |
| `crates/duckpond/src/plan.rs` | `HookPosition`, `create_hook`, `STAGES` |
| `crates/duckspec/src/cmd/create.rs` | Clap flags, skeleton seed, execute plan |
| `crates/duckspec/src/cmd/template.rs` | Load/inject hooks (already correct) |
| `duckspec/caps/cli/hooks/` | Spec/doc for create, injection, stages |
```

## Vocabulary rename

Rename create/plan only; template path format already matches the target.

```
| Surface | From | To |
| --- | --- | --- |
| Enum | `Pre` / `Post` | `Before` / `After` |
| `as_str` | `"pre"` / `"post"` | `"before"` / `"after"` |
| Clap | `--pre` / `--post` | `--before` / `--after` |
| Path | `hooks/<stage>-pre.md` | `hooks/<stage>-before.md` |
| | `hooks/<stage>-post.md` | `hooks/<stage>-after.md` |
| Skeleton | `# Explore - Pre` | `# Explore - Before` |
```

Clap keeps an exclusive position group: exactly one of `--before` or `--after`. No
aliases. Help text stage list matches `STAGES` (include `backfill` and the full set). Plan
unit tests assert the new paths and variants.

Failure modes unchanged: unknown stage → `UnknownStage`; existing file →
`HookExists { path }` with the new path.

## Skeleton seed

Create still writes a non-empty H1 for editor/agent convenience:

```markdown
# Explore - Before
```

```markdown
# Spec - After
```

Injection does not require an H1; body is freeform and emitted verbatim. The seed is
create-time only, not a file schema.

## Naming source of truth

Convention + capability, not a shared helper:

- Plan owns create-time naming via `HookPosition::as_str()`.

- Template keeps its existing `"before"` / `"after"` path literals (already correct).

- `cli/hooks` states the shared filename rule so create and template cannot drift in the
  product description.

Rejected for this change: exporting path helpers or forcing template to call `plan` for
two fixed suffixes.

Template still does not validate stage against `STAGES`. Create alone gates known stages;
unknown template names already fail when the stock template is missing.

## Capability: `cli/hooks`

New full capability at `duckspec/caps/cli/hooks/` (`spec.md` + `doc.md`).

Three requirement groups:

```
| Requirement | Behavior owned |
| --- | --- |
| **Create hook** | Exactly one of `--before`/`--after`; path `hooks/<stage>-{before\|after}.md`; skeleton seed; unknown stage fails; existing file fails; creates parent `hooks/` as needed |
| **Template injection** | Read `hooks/<stage>-before.md` / `-after.md`; trim; empty/whitespace ⇒ absent; present body under `## Before write` / `## After write`; drop placeholder when absent; body verbatim |
| **Known stages** | Create accepts only `STAGES`; that list is the known hook stages for scaffolding |
```

`cli/stock-content` is left alone (no ownership transfer edit required). Codices and
README need no design-driven rewrite. No audit rules for hook files.

## Compatibility

Hard break for anyone using `--pre`/`--post` or `*-pre.md`/`*-post.md`. In-repo hook
`hooks/apply-after.md` already uses the new naming. No migration tooling.

## Settled choices

- Enum and flags rename fully to Before/After (not keep Pre/Post with different strings).
- Skeleton option A: `# {Stage} - Before|After`.
- No shared path API in duckpond for this change.
- Cap under `cli/hooks` with three-way requirement split; stock-content untouched.
