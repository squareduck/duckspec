# Document parser

Parses a generic markdown document from a Layer 1 element stream into a typed `Document`
artifact. The doc parser is the lenient counterpart to `parse/spec` — it owns the same
preamble (H1, summary, description) but imposes no schema on the section tree.

## Used by

```
| Artifact          | Where                                  |
|-------------------|----------------------------------------|
| Capability doc    | caps/<path>/doc.md                     |
| Codex entry       | codex/<slug>.md                        |
| Project overview  | project.md                             |
| Proposal          | changes/<name>/proposal.md             |
| Design            | changes/<name>/design.md               |
```

Every freeform-content artifact in duckspec routes through `parse_document`. That makes
the parser low-validation-by-design: each of these formats has its own conventional H2
layout, but none of them has a hard schema beyond the H1 + summary preamble.

## Pipeline

```
  Vec<Element>
       │
       ▼
  ┌──────────────────┐
  │ parse_document   │ ──→ Document { title, summary, description, sections }
  └──────────────────┘
       │
       └─→ Vec<ParseError>  (only the shared L2 preamble errors)
```

The parser walks the element stream once: pulls the H1, pulls the summary, collects
description blocks until the first heading, then recurses into `parse_sections` to build
the section tree.

## Section tree

Each `Section` carries:

```
| Field         | Meaning                                                      |
|---------------|--------------------------------------------------------------|
| heading       | the heading text, without leading hashes                     |
| level         | the heading level (2..=6)                                    |
| heading_span  | byte span of the heading element                             |
| body          | block elements between this heading and the next             |
| children      | nested sections at deeper levels                             |
```

The recursion rule is straightforward: every heading at the current `min_level` opens a
new sibling at this level, then recurses into `parse_sections(level +
1)` to claim deeper
headings as children. A heading shallower than `min_level` returns control to the caller,
where it'll be picked up by an ancestor's recursion.

```
# Title
## A             ← top-level section A
### A.1          ← child of A
#### A.1.x       ← child of A.1
## B             ← top-level section B (closes A)
```

Bodies hold any non-heading elements between consecutive headings and are preserved
verbatim. The parser does not interpret list items, paragraphs, or code blocks beyond
passing them through — downstream tooling (rendering, formatting) handles those.

## Validation boundaries

The doc parser intentionally has only three error variants — all shared with `parse/spec`:

- `ContentBeforeH1` — first element is not an H1
- `MissingH1` — empty input
- `MissingSummary` — H1 not followed by a paragraph

It does not reject H4+ headings, mismatched body content, or any other structural quirk.
Strict-shape artifacts (specs, deltas, steps) have dedicated parsers; everything else uses
this lenient one.
