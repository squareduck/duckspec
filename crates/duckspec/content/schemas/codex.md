# Codex schema

A codex entry is **cross-cutting project knowledge** no single capability owns:
architecture, glossaries, design philosophy, engineering conventions. Edited
directly under `duckspec/codex/` - no change, delta, or archive lifecycle.

## Structure

```markdown
# <Entry Title>

<1-2 sentence summary>

<body>
```

Body is freeform markdown (headings, lists, tables, diagrams, prose).

## Rules

- H1 title is required
- A non-empty summary paragraph follows the H1 directly
- No structural rules on the body beyond parseable markdown
- Path under `duckspec/codex/`: kebab-case segments, no whitespace; may nest
  (`codex/domain/billing.md`)
- One topic per entry file

## Quality

- **Placement.** If one capability owns it, put it in that capability’s doc
  instead. If it is always-true project constitution, prefer `project.md`.
- **Focus.** One entry per topic - glossary, architecture, and conventions are
  separate entries, not one dump.
- **Index-ready summary.** `ds index` shows the summary; make it orient a
  scanner without opening the file.
- **Durable.** Strip session-specific context; write for a reader months later.
- Body markdown follows `style` (load only if not already in context).

## Formatting

After write or edit: `ds format <path>`. Presentation (tables, fences, diagrams)
follows `style` - load only if not already in context.

## Example

```markdown
# Error handling conventions

Libraries use typed `thiserror` enums; binaries use `anyhow` at the boundary.

## Library crates

Define per-module error enums with enough context to diagnose without the call
site.

## Binary crates

Use `anyhow::Result` at the application edge. Attach `.context()` at layer
crossings.
```
