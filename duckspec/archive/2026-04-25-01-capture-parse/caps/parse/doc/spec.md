# Document parser

Parses a generic markdown document from a Layer 1 element stream into a typed `Document`
artifact: H1 title, summary paragraph, optional description, and a recursive section tree
built from H2 and deeper headings. Used for capability docs, codex entries, `project.md`,
proposals, and designs.

The parser shares the H1/summary/description preamble code path with `parse/spec` — common
L2 errors (`ContentBeforeH1`, `MissingH1`, `MissingSummary`) are pinned on `parse/spec`'s
scenarios and not duplicated here. Doc-side validation is otherwise lenient: any heading
nesting is allowed and section bodies accept arbitrary block content.

## Requirement: Document structure

A document SHALL consist of, in order: an H1 heading carrying the title, a single
paragraph carrying the summary, an optional description (any blocks before the first
heading), and a section tree built from H2-and-deeper headings. Body content of any
section is unconstrained markdown — paragraphs, lists, code blocks, and tables are all
valid.

> test: code

### Scenario: Minimal document parses successfully

- **GIVEN** a source containing only an H1 title and a summary paragraph
- **WHEN** the document is parsed
- **THEN** a `Document` artifact is produced
- **AND** the `sections` list is empty

## Requirement: Section tree

Headings at H2 or deeper SHALL produce `Section` nodes carrying their heading text, level,
body elements, and child sections. A heading deeper than the current section's level SHALL
nest as a child of that section. A heading at or shallower than the current section's
level SHALL close the current section and attach to the appropriate ancestor.

> test: code

### Scenario: Document with sibling H2 sections parses into a flat section list

- **GIVEN** a source with multiple H2 sections at the same level
- **WHEN** the document is parsed
- **THEN** the `sections` list contains one entry per H2
- **AND** each section's `children` list is empty

### Scenario: Document with nested headings produces a parent-child section tree

- **GIVEN** a source with an H2 followed by H3 children, then another H2 with H3 children
- **WHEN** the document is parsed
- **THEN** each H2 appears in the top-level `sections` list
- **AND** each H3 appears as a child of the preceding H2

### Scenario: Section tree captures headings nested four levels deep

- **GIVEN** a source containing an H2 → H3 → H4 → H5 chain
- **WHEN** the document is parsed
- **THEN** the section tree contains the full chain as nested `children`
- **AND** each level's `level` field matches its heading level
