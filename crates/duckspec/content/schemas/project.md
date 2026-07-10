# Project schema

`duckspec/project.md` is the **project constitution**: what the project is and
durable high-level facts agents should always consider. Optional - omit when
unused. Edited directly; not carried through a change.

## Structure

```markdown
# <Project Name>

<1-2 sentence summary>

<body>
```

Body is freeform markdown (headings, lists, tables, diagrams, prose).

## Rules

- H1 title is required
- A non-empty summary paragraph follows the H1 directly
- No structural rules on the body beyond parseable markdown
- At most one file: `duckspec/project.md`
- Edited in place - no deltas, no change folder copy

## Quality

- **What the project is.** Identity, purpose, and durable high-level facts -
  stack, shape, standing principles - not a change log and not a backlog of
  what is out of scope for some future effort.
- **Always true.** Content should still hold next year; session and feature
  narratives belong elsewhere.
- **Not agent config.** Do not duplicate `AGENTS.md` / harness instructions;
  this is project knowledge agents should *consider*, not runtime prompt wiring.
- **Lean.** Prefer a short constitution. Past about two screens, split lasting
  topics into codex entries.
- Body markdown follows `style` (load only if not already in context).

## Formatting

After write or edit: `ds format <path>`. Presentation follows `style` - load only
if not already in context.

## Example

```markdown
# acme-api

REST API for the Acme suite - Rust, explicit errors, small crates.

## Shape

- Library-first crates; thin CLI and HTTP adapters
- Filesystem is the source of truth - no frontmatter or sidecar metadata

## Principles

- Typed errors in libraries; `anyhow` only at binary boundaries
- Spec-backed behavior for user-visible contracts
```
