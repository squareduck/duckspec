# Schema structure

Canonical section shape for **on-disk artifact schemas** (proposal, design, spec, step,
review, and the rest). Principles live in `template-and-schema-authoring`. The shared
markdown guide (`style`) is not an artifact schema and does not use this skeleton.

## Skeleton

Sections appear in this order. Every artifact schema has every section.

```markdown
# <Title> schema

<1-2 sentence summary>

## Structure

## Rules

## Quality

## Formatting

## Example
```

The H1 names the artifact (e.g. `Proposal schema`). The summary orients agents loading the
schema via `ds schema <name>`.

## Structure

A fenced skeleton of the artifact: headings, markers, and placeholders that define shape.
Enough to draft from; not a full worked document.

## Rules

Mechanical constraints that define validity: required headings, markers, ordering,
cardinality, naming, and anything `ds check` or a parser can enforce. Stated as direct
requirements on the artifact text. Process, voice, and CLI workflow stay out - those are
template concerns.

## Quality

Judgment principles for a *good* instance of this artifact: focus, taste, lightest touch,
cold-reader clarity, and similar. Short bullets. Not a second Structure section and not a
process script for the agent. Presentation taste (tables, diagrams, prose) defers to
`style` with a single pointer - do not restate the style guide here.

## Formatting

Mechanical canonicalize after edit: `ds format <path>`, plus any artifact-specific
mechanical notes. Point at `ds schema style` for markdown presentation (tables, fences,
diagrams) with **load only if not already in context**. Keep this section short so a
just-in-time schema load stays light.

## Example

At most one canonical illustration of a valid artifact of this kind. Prefer a small
skeleton or one multi-part combination that rules alone underspecify. When Structure and
Rules are enough, a minimal skeleton still fills this section so every schema stays
complete and scannable. The example’s body markdown follows `style`.
