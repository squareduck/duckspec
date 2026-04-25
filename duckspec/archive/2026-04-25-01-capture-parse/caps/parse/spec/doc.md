# Capability spec parser

Parses a capability spec (`spec.md`) from a Layer 1 element stream into a typed `Spec`
artifact. Specs are the most consequential artifact in duckspec — every scenario marked
`test: code` becomes a maintenance commitment.

## Pipeline

```
  Vec<Element>
       │
       ▼
  ┌──────────────┐
  │  parse_spec  │ ──→ Spec { title, summary, description, requirements }
  └──────────────┘
       │
       └─→ Vec<ParseError> on validation failure
```

The parser walks the element stream once with an explicit cursor, building each
`Requirement` as it encounters an H2, then each `Scenario` as it encounters an H3 inside
that requirement. Errors accumulate into a `Vec<ParseError>` rather than short-circuiting
— a single parse pass surfaces every problem in the file.

## Document layout

```
# <Title>                         ← required H1
                                  ← required summary paragraph
<summary paragraph>

<optional description>            ← any blocks before first H2

## Requirement: <Name>            ← H2 with literal prefix
                                  ← optional normative prose (any blocks)
<prose paragraphs>

> test: code                      ← optional requirement-level test marker

### Scenario: <Name>              ← H3 with literal prefix

- **GIVEN** ...                   ← GWT clause list
- **WHEN** ...
- **THEN** ...

> test: code                      ← optional scenario-level test marker
```

H4 and deeper headings are forbidden anywhere in the document. The body of a scenario is
exactly one unordered list of GWT clauses optionally followed by a test marker —
paragraphs, headings, and other content are rejected.

## GWT phase machine

Clauses progress through three phases. `AND` continues whichever phase came immediately
before; the schema imposes no ordering beyond "at least one WHEN and one THEN."

```
   ┌───────┐  GIVEN     ┌───────┐  WHEN     ┌──────┐  THEN     ┌──────┐
   │ Start │ ─────────→ │ Given │ ────────→ │ When │ ────────→ │ Then │
   └───────┘            └───────┘           └──────┘           └──────┘
                            │                  │                 │
                            │ AND              │ AND             │ AND
                            ▼                  ▼                 ▼
                         (Given)            (When)             (Then)
```

Any other transition is `GwtClauseOutOfOrder`. The three branches that exercise distinct
error arms:

- `AND` in `Start` — no preceding clause to continue.
- `THEN` in `Start` or `Given` — no `WHEN` has occurred.
- `GIVEN` in `When` or `Then` — phase has already advanced.

## Test markers

A spec author tags every requirement or scenario with one of three markers:

```
| Marker            | Meaning                                                |
|-------------------|--------------------------------------------------------|
| test: code        | scenario must be covered by automated tests            |
| manual: <reason>  | scenario verified by humans; reason explains why       |
| skip: <reason>    | scenario intentionally untested; reason explains why   |
```

A `> test: code` blockquote may be followed by zero or more `> - <path>` lines that record
backlinks to the test code. Backlinks are resolved by `ds sync` and checked by `ds audit`;
a `test: code` scenario with no backlinks is a backlog item.

If a scenario carries no marker, the parser inherits the requirement's marker. If neither
the scenario nor the requirement has one, parsing fails with `UnresolvedTestMarker` —
every scenario MUST resolve to a marker.

## Error catalogue

```
| Variant                       | Triggered by                                                            |
|-------------------------------|-------------------------------------------------------------------------|
| ContentBeforeH1               | first element is not an H1 heading                                      |
| MissingH1                     | source has no elements                                                  |
| MissingSummary                | H1 not followed by a paragraph                                          |
| HeadingTooDeep                | any H4+ heading anywhere in the document                                |
| InvalidRequirementPrefix      | H2 content does not begin with `Requirement: `                          |
| RequirementNameColon          | requirement name (after the prefix) contains `:`                        |
| EmptyRequirement              | requirement has no prose and no scenarios                               |
| InvalidScenarioPrefix         | H3 content does not begin with `Scenario: `                             |
| MissingWhen                   | scenario body has no `WHEN` clause                                      |
| MissingThen                   | scenario body has no `THEN` clause                                      |
| UnexpectedScenarioContent     | scenario body contains a non-list-item, non-blockquote element          |
| GwtClauseOutOfOrder           | clause appears in a phase that disallows it                             |
| InvalidGwtKeyword             | list-item begins with `**X**` for some X not in {GIVEN, WHEN, THEN, AND} |
| InvalidTestMarker             | blockquote in a marker position does not match a known prefix           |
| UnresolvedTestMarker          | scenario has no marker and parent requirement has no marker             |
```

Errors carry the source `Span` of the offending element so `miette` can render diagnostics
with line numbers and underlines.
