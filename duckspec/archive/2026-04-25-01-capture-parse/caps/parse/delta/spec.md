# Delta artifact parser

Parses a delta artifact (`spec.delta.md`, `doc.delta.md`) from a Layer 1 element stream
into a typed `Delta` artifact: marker-prefixed root with optional summary, description,
and entries; per-marker child rules; canonical sort; mandatory whitespace between marker
and heading text.

Deltas describe targeted modifications to an existing capability spec or doc. Each entry
carries one of five markers — `=` rename, `-` remove, `~` replace, `@` anchor, `+` add —
that determines what the entry does and what kind of children it accepts.

## Requirement: Delta document structure

A delta SHALL consist of, in order: an H1 heading carrying a marker (any marker except
`+`) and the title, an optional summary paragraph, an optional description (any blocks
before the first H2), and a sequence of H2 entries each carrying a marker. Top-level H2
entries SHALL be sorted into canonical order (`=` → `-` → `~` → `@` → `+`) regardless of
source order. Operation children of `@` entries SHALL be sorted by the same canonical
order.

> test: code

### Scenario: Delta with a single Add entry parses successfully

- **GIVEN** a source containing an anchor H1 and one `+` H2 entry with content children
- **WHEN** the delta is parsed
- **THEN** a `Delta` artifact is produced
- **AND** the entries list contains one `Add` entry
- **AND** the entry's children are `Content` sections

### Scenario: Delta with mixed-marker entries is sorted into canonical order

- **GIVEN** a source whose H2 entries appear in non-canonical order (for example `@ + =`)
- **WHEN** the delta is parsed
- **THEN** the entries list is sorted into canonical order (`=` → `-` → `~` → `@` → `+`)

### Scenario: Delta with rename and anchor entries parses with new-name extraction

- **GIVEN** a source containing a rename H2 entry whose body's first paragraph carries the
  new name, and an anchor H2 entry with operation children

- **WHEN** the delta is parsed

- **THEN** the rename entry's `rename_to` field is populated with the new name

- **AND** the anchor entry's children are `Operations`

## Requirement: Marker rules

Every H1, H2, and H3 heading in a delta SHALL carry one of the recognized marker
characters (`+ - ~ @ =`) as its first character, followed by exactly one ASCII space, then
the heading text. A marker without a following space SHALL be rejected. Specific markers
are valid only at specific levels and positions: `+` SHALL NOT appear on H1; `@` SHALL NOT
appear on H3; children of `~` and `+` entries SHALL NOT carry markers.

> test: code

### Scenario: Marker without a following space raises MarkerMissingSpace

- **GIVEN** a source containing a heading whose marker character is not followed by an
  ASCII space

- **WHEN** the delta is parsed

- **THEN** parsing fails with `ParseError::MarkerMissingSpace`

### Scenario: Add marker on H1 raises AddOnH1

- **GIVEN** a source whose H1 heading carries the `+` marker
- **WHEN** the delta is parsed
- **THEN** parsing fails with `ParseError::AddOnH1`

### Scenario: H2 without a marker raises MissingDeltaMarker

- **GIVEN** a source containing an H2 whose first character is not one of `+ - ~ @ =`
- **WHEN** the delta is parsed
- **THEN** parsing fails with `ParseError::MissingDeltaMarker`

### Scenario: Anchor marker on H3 raises AnchorOnH3

- **GIVEN** a source whose H3 heading under an anchor entry carries the `@` marker
- **WHEN** the delta is parsed
- **THEN** parsing fails with `ParseError::AnchorOnH3`

### Scenario: Marker on a content child raises MarkerOnContentChild

- **GIVEN** a source containing an H3 child of a `~` or `+` entry whose first character is
  a marker

- **WHEN** the delta is parsed

- **THEN** parsing fails with `ParseError::MarkerOnContentChild`

## Requirement: Per-marker entry semantics

Each marker imposes specific structural rules on its entry's body and children:

- `@` (anchor) — children are operation entries that each carry their own marker
  (`DeltaChildren::Operations`).

- `~` (replace) and `+` (add) — children are plain content sections without markers
  (`DeltaChildren::Content`).

- `-` (remove) — body MUST be empty; no children allowed.

- `=` (rename) — body's first paragraph carries the new name; no children allowed.

> test: code

### Scenario: Remove with body or rename without new-name line raises the relevant invariant

- **GIVEN** a source containing a `-` entry with a non-empty body and a `=` entry without
  a new-name line

- **WHEN** the delta is parsed

- **THEN** parsing fails with `ParseError::NonEmptyRemoveBody` for the remove entry

- **AND** parsing fails with `ParseError::InvalidRenameEntry` for the rename entry
