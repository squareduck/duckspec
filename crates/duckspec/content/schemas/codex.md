# Codex schema

A codex entry is durable project knowledge no single capability owns and that
is more detailed than the short high-level orientation in `project.md`.
Architecture, testing strategy, UI language, engineering conventions,
glossaries, and research records belong here when they remain useful across
the project.

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
- One cohesive durable subject and reader purpose per entry

## Quality

- **Placement.** `project.md` stays a short high-level description of what the
  project is. A capability doc owns knowledge scoped to that capability. Codex
  owns all other durable project knowledge.
- **Cohesive ownership.** Each entry has one durable subject and reader purpose.
  Prefer one authoritative entry over fragments that must always be read
  together. Split when subjects evolve independently or serve different
  readers.
- **Tree quality.** Consolidate overlapping entries, reconcile contradictions,
  relocate misplaced material, and remove superseded guidance. Judge the
  complete codex tree, not only the file currently being edited.
- **Index-ready summary.** `ds index` shows the summary; make it orient a
  scanner without opening the file.
- **Current durable truth.** Synthesize the useful result, not the history of
  the conversation. Preserve rationale when readers need it to apply the
  guidance correctly.
- **Grounded and navigable.** Terminology agrees with project orientation,
  capabilities, source, and tests. Use headings, tables, diagrams, examples,
  and code when they make the knowledge easier to use.
- **Settled by default.** Do not preserve unresolved chat unless the entry is
  explicitly a research record whose purpose includes known unknowns.
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
