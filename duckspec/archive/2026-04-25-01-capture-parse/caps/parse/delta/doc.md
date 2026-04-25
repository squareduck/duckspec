# Delta artifact parser

Parses a delta artifact from a Layer 1 element stream into a typed `Delta` artifact.
Deltas express targeted modifications to a target spec or doc and are the working
artifacts of any in-flight change.

## Marker semantics

Five marker characters drive every delta operation:

```
| Marker | Name    | Where           | Body              | Children                     |
|--------|---------|-----------------|-------------------|------------------------------|
| =      | Rename  | H1, H2, H3      | new-name line     | none                         |
| -      | Remove  | H1, H2, H3      | empty             | none                         |
| ~      | Replace | H1, H2, H3      | optional content  | Content sections (unmarked)  |
| @      | Anchor  | H1, H2 (not H3) | optional content  | Operations (each marked)     |
| +      | Add     | H2, H3 (not H1) | optional content  | Content sections (unmarked)  |
```

The marker on the H1 sets the operation that targets the document as a whole; H2 and H3
entries scope the operation to specific named sections.

## Heading syntax

Every heading in a delta carries a marker followed by exactly one ASCII space followed by
the heading text:

```
# @ Authentication              ← H1: anchor on the whole spec
## + Requirement: Two-factor    ← H2: add a new requirement
### + Scenario: Enrollment      ← H3: add a new scenario under that requirement
```

The single-space rule is strict — `# @Authentication` raises `MarkerMissingSpace`.
Multiple spaces are accepted; `ds format` normalizes them to one space at write time.

## Canonical sort

Entries are sorted at parse time so the artifact's structure is independent of authoring
order:

```
order: =  →  -  →  ~  →  @  →  +
```

The same sort applies recursively to the operation children of `@` entries. Source order
is irrelevant — readers can rely on a stable ordering when diffing or rendering deltas.

## Child-rule decision tree

When the parser encounters an entry's H3 children, the parent's marker determines which
path it takes:

```
  parent marker
       │
       ├── @ ──→ parse_operation_children
       │           Each H3 must carry a marker.
       │           Children become DeltaChildren::Operations.
       │
       ├── ~ ──┐
       │      ├──→ parse_content_children
       ├── + ──┘    Each H3 must NOT carry a marker.
       │           Children become DeltaChildren::Content.
       │
       ├── = ──→ no children expected
       │           DeltaChildren::Content(vec![])
       │
       └── - ──→ no children expected
                  DeltaChildren::Content(vec![])
```

A marker on a content child (under `~` or `+`) raises `MarkerOnContentChild`. An `@`
marker on a child of an anchor (i.e. on an H3) raises `AnchorOnH3`.

## Rename entries

Rename is the only marker whose body has a positional meaning: the first paragraph after
the heading is the new name.

```
## = Email-password login
Email/password authentication
```

The first paragraph (`Email/password authentication`) is captured as the entry's
`rename_to` field. A rename entry with no body raises `InvalidRenameEntry`.

## Error catalogue

Delta-specific variants:

```
| Variant              | Triggered by                                                 |
|----------------------|--------------------------------------------------------------|
| MissingDeltaMarker   | a heading's first character is not one of `+ - ~ @ =`        |
| MarkerMissingSpace   | a marker character is not followed by an ASCII space         |
| AddOnH1              | the H1 heading carries the `+` marker                        |
| AnchorOnH3           | an H3 heading carries the `@` marker                         |
| MarkerOnContentChild | an H3 child of a `~` or `+` parent carries any marker        |
| NonEmptyRemoveBody   | a `-` entry has body content                                 |
| InvalidRenameEntry   | a `=` entry has no body, or its first paragraph is empty     |
```

Shared L2 errors (`ContentBeforeH1`, `MissingH1`, `HeadingTooDeep`) are documented in
`parse/spec` and apply here unchanged.
