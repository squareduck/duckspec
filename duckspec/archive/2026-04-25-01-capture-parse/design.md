# Capture parse subsystem — Design

Five capabilities under `caps/parse/` paired with one parser tightening (mandatory space
after delta marker) and a new error fixtures convention.

## Approach

```
                       ┌──────────────────────────────────────┐
                       │ caps/parse/                          │
                       └──────────────────────────────────────┘
                              │
       ┌──────────────────────┼─────────────────────────────────┐
       ▼                      ▼                                 ▼
  ┌──────────┐         ┌─────────────┐                ┌─────────────────┐
  │ elements │         │   spec      │                │  doc            │
  │  (L1)    │         │   (L2)      │                │  (L2)           │
  └──────────┘         └─────────────┘                └─────────────────┘
                              │                                 │
                              ▼                                 ▼
                       ┌─────────────┐                ┌─────────────────┐
                       │  delta      │                │  step           │
                       │  (L2)       │                │  (L2)           │
                       └─────────────┘                └─────────────────┘
```

Two layers, two-phase capture:

- **Layer 1** (`parse_elements`) gets one cap with mostly-prose contract pinning;
  scenarios cover element kinds, span semantics, classification rules, and the documented
  "any input produces a valid sequence" guarantee.

- **Layer 2** (spec/doc/delta/step) gets four caps each with happy-path + error scenarios.
  Common L2 errors (`ContentBeforeH1`, `MissingH1`, `MissingSummary`, `HeadingTooDeep`)
  are pinned once on `parse/spec` (the most-exercised parser); `parse/{doc,delta,step}`
  reference shared behavior in their docs and only pin parser-specific errors.

One parser tightening sits beside the capture: `parse_marked_heading` will require at
least one ASCII space between the marker char and the heading text, closing the only
behavioral ambiguity. Error fixtures live in
`tests/fixtures/<artifact>/errors/<variant>.md` and the test reads them via
`parse_fixture(...).expect_err(...)` with snapshot assertions over the `Vec<ParseError>`
debug output.

## Capability layout

```
caps/parse/
├── elements/
│   ├── spec.md      ← element model, span semantics, classification rules
│   └── doc.md       ← Layer 1 architecture, infallibility contract
├── spec/
│   ├── spec.md      ← H1/summary/requirements/scenarios + GWT machine + markers + shared L2 errors
│   └── doc.md       ← spec artifact pipeline overview
├── doc/
│   ├── spec.md      ← H1/summary + recursive section tree
│   └── doc.md       ← used-by list (capability docs, codex, project.md, proposals, designs)
├── delta/
│   ├── spec.md      ← marker entries + canonical sort + rename + per-marker child rules + whitespace rule
│   └── doc.md       ← marker semantics table, child-rule decision tree
└── step/
    ├── spec.md      ← named sections + checkboxes + @spec/@step refs + subtasks
    └── doc.md       ← Tasks vs Prerequisites distinction, ref-syntax overview
```

No umbrella `caps/parse/` cap. The parent path is a folder only — the five leaves are
self-contained, and a meta-cap covering "the parsing subsystem" would duplicate what each
leaf's `doc.md` already conveys.

## Parser tightening: `MarkerMissingSpace`

The current `parse_marked_heading` accepts both `# @ Foo` and `# @Foo` silently. After
this change, only forms with at least one space between marker and text parse; everything
else emits a new error variant.

```rust
#[error("delta marker must be followed by a space")]
MarkerMissingSpace {
    #[label("missing space after marker")]
    span: Span,
},
```

Slot in `error.rs` immediately after `MissingDeltaMarker` (line 128 area), inside the
"Delta errors" comment block. Add to the `ParseError::span()` match arm.

The function changes shape:

```rust
fn parse_marked_heading(
    heading: &str,
    span: Span,
    errors: &mut Vec<ParseError>,
) -> Option<(DeltaMarker, String)> {
    let trimmed = heading.trim();
    let mut chars = trimmed.chars();
    let first = chars.next()?;

    let Some(marker) = DeltaMarker::from_char(first) else {
        errors.push(ParseError::MissingDeltaMarker { span });
        return None;
    };

    let rest = chars.as_str();
    if !rest.starts_with(' ') {
        errors.push(ParseError::MarkerMissingSpace { span });
        return None;
    }

    let title = rest.trim().to_string();
    Some((marker, title))
}
```

Rule: the byte directly after the marker char must be `0x20`. Tabs, no-space all reject.
Multiple spaces remain accepted (the rest is `trim()`-ed and `ds format` will canonicalize
to one space at write time).

```
# @ Foo       ✓ canonical
# @  Foo      ✓ accepted (formatter canonicalizes to single space)
# @Foo        ✗ MarkerMissingSpace
# @\tFoo      ✗ MarkerMissingSpace
# @           ✗ MarkerMissingSpace (no separator)
```

## Layer 1 contract pinning

`parse_elements` is documented as infallible. The spec scenarios pin specific
manifestations as `test: code`:

```rust
// existing inline tests in src/parse.rs already cover:
//   - empty input → []
//   - single heading → Heading element
//   - heading levels 1..=6 distinct
//   - paragraphs (single-line, multi-line, blank-line-separated)
//   - code blocks (preserve fence + content + blank lines)
//   - lists (bullet, numbered, nested, continuation, loose, double-digit)
//   - block quotes (line, with list-formatted content)
//   - mixed content
//   - heading terminates paragraph
//   - span offsets correct
//   - false positives: #hashtag, 1.no-space, 1.0 version
//   - bullet vs numbered marker distinction

// new tests to add:
//   - unclosed_code_block_flushes_at_eof  (pins infallibility manifestation)
//   - code_fence_with_info_string         (pins ```rust preservation)
//   - block_quote_terminates_list_item    (pins test-marker boundary)
```

These three new tests add ~10 LOC each in the existing `#[cfg(test)] mod tests` block of
`src/parse.rs`. No new files.

## Error fixture convention

New layout per Layer 2 parser:

```
crates/duckpond/tests/fixtures/
├── spec/
│   ├── minimal.md
│   ├── multi_requirement.md
│   ├── ...
│   └── errors/                  ← NEW
│       ├── content_before_h1.md
│       ├── heading_too_deep.md
│       ├── invalid_requirement_prefix.md
│       └── ...
├── doc/
│   ├── ...
│   └── deep_recursion.md        (no errors/ subdir — no doc-specific errors to capture)
├── delta/
│   ├── ...
│   └── errors/
│       ├── add_on_h1.md
│       ├── marker_missing_space.md
│       ├── anchor_on_h3.md
│       └── ...
└── step/
    ├── ...
    └── errors/
        ├── unknown_section.md
        ├── tasks_missing_or_empty.md
        └── ...
```

Each error fixture is a minimal `.md` file that triggers exactly one variant (or one
cluster of variants in the case of `tasks_missing_or_empty.md`).

The tests use this pattern:

```rust
#[test]
fn err_content_before_h1() {
    let errors = parse_fixture("errors/content_before_h1.md")
        .expect_err("should fail");
    insta::assert_debug_snapshot!(errors);
}
```

The snapshot covers both the variant set and the spans — a future refactor that preserves
error variants but drifts span ranges will surface as a snapshot diff.

## Test scenario count

```
┌──────────────────┬──────────┬──────────┬───────┐
│ Cap              │ Existing │ New      │ Total │
├──────────────────┼──────────┼──────────┼───────┤
│ parse/elements   │       28 │        3 │    31 │
│ parse/spec       │        5 │        9 │    14 │
│ parse/doc        │        3 │        1 │     4 │
│ parse/delta      │        3 │        6 │     9 │
│ parse/step       │        3 │        6 │     9 │
├──────────────────┼──────────┼──────────┼───────┤
│ TOTAL            │       42 │       25 │    67 │
└──────────────────┴──────────┴──────────┴───────┘
```

All 67 scenarios marked `test: code`. Zero `manual:`. Zero `skip:`.

## Decisions

- **Whitespace rule: ≥1 ASCII space, no tabs.** The parser requires at least one space
  between marker and text. Multiple spaces are accepted; `ds format` normalizes to one.
  Alternatives: (a) exactly one space — rejected because it forces every author tool to
  canonicalize before saving, including IDEs and pre-commit hooks; (b) any whitespace
  separator including tabs — rejected because tabs in headings are unusual and often
  signal a paste-from-elsewhere accident worth flagging.

- **No umbrella `caps/parse/` cap.** Each L1/L2 cap is self-contained; a meta-cap would
  duplicate the per-leaf docs. Alternative: parent `parse/` cap with high-level Layer 1 vs
  Layer 2 architecture content — rejected because that material fits naturally in
  `parse/elements/doc.md` (Layer 1) plus brief cross-references in each Layer 2 doc.

- **Shared L2 errors live on `parse/spec` only.** `ContentBeforeH1`, `MissingH1`,
  `MissingSummary`, `HeadingTooDeep` are common to all four Layer 2 parsers but exercise
  the same code shape. Pinning them on the most-complex parser (spec) is enough;
  doc/delta/step docs cite spec's coverage rather than duplicating. Alternative: pin each
  shared variant on every artifact parser — rejected as 4× redundant snapshots that all
  break together when the variant changes.

- **L1 manifestations pinned as `test: code`.** Unclosed code blocks, fence info-string
  preservation, and block-quote-terminates-list-item are concrete manifestations of the
  documented infallibility contract. Pinning makes the contract enforceable. Alternative:
  leave L1 contract as prose in `doc.md` — rejected because a future "fail on unclosed
  fence" change could land without spec drift, breaking every Layer 2 parser that relied
  on the guarantee.

## Risks

- **Tightening `parse_marked_heading` could break existing valid inputs that omit the
  space.** → Audited: every fixture in `tests/fixtures/` and every delta example in the
  README uses the canonical `<marker> <text>` form. The duckspec repo itself has no `.md`
  files affected (no caps yet). Mitigation: if external repos break post-release, document
  the migration as "search for H1/H2/H3 starting with `[+\-~=@]\S` and add a space."

- **Error fixture proliferation could clutter `tests/fixtures/`.** → Mitigation: isolate
  error fixtures in an `errors/` subdirectory per artifact; happy-path fixtures stay at
  the top of each artifact's directory. Naming pattern `<variant_snake_case>.md` makes the
  mapping to `ParseError` variants obvious; combined fixtures
  (`tasks_missing_or_empty.md`, `remove_and_rename_invariants.md`) reduce file count where
  one fixture can exercise two variants without ambiguity.

## Open questions

None. Whitespace contract is settled (≥1 ASCII space, no tabs); error fixture layout is
decided; no umbrella cap; L1 manifestations pinned. Ready for `/ds-spec`.
