# Codex entry schema

A codex entry captures **cross-cutting knowledge** that spans capabilities or
stands outside them: architecture overviews, domain glossaries, design
philosophy, engineering conventions.

Codex entries are edited directly — no deltas, no change workflow, no archive
lifecycle.

## Structure

```markdown
# <Entry Title>

<1-2 sentence summary>

<freeform markdown content>
```

## Rules

- H1 title is required.
- A summary paragraph directly follows the H1.
- The body may contain any markdown.
- No structural validation beyond the H1 and summary.
- Codex entry paths use kebab-case, no whitespace.

## Quality

- **Write codex entries for knowledge that no single capability owns.** If it
  belongs to one capability, put it in that capability's doc instead.
- Keep entries focused. One entry per topic. A glossary is one entry;
  architecture is another.
- Summaries are used by `ds index` — make them informative enough to orient a
  reader scanning the index.

## Formatting

After writing or updating this artifact, run `ds format <path>` to apply
canonical formatting (line wrap, indentation, blank lines).

Use fenced code blocks for tables and diagrams; add a `<language>` tag to
fences that contain real code.

## Example

```markdown
# Error handling conventions

All crates in this workspace use a two-tier error strategy: typed enums in
libraries, anyhow wrapping in binaries.

## Library crates

Use `thiserror` to define per-module error enums. Each variant carries enough
context to diagnose the failure without access to the call site.

## Binary crates

Use `anyhow::Result` at the application boundary. Attach context with
`.context()` at each layer crossing.
```
