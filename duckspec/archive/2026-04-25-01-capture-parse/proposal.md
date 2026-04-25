# Capture parse subsystem

Capture the existing markdown parsing layer of `duckpond` as five capabilities under
`caps/parse/`, the first slice of the duckspec self-host backfill.

## Motivation

The parsing layer is the foundation everything else in `duckpond` builds on — Layer 1
(`parse_elements`) feeds Layer 2 artifact parsers (spec / doc / delta / step), and every
downstream subsystem (rendering, merge, audit) consumes their output. Pinning parser
behavior as `test: code` scenarios is high-leverage: it stops format drift, makes the
artifact contracts inspectable, and establishes the cap vocabulary (`Element`, `Span`,
`Spec`, `Delta`, `Step`, `Document`) that every later backfill slice will reference. As
the first capture, it also exercises the self-host workflow end-to-end and shows what a
good first slice looks like.

## Scope

```
caps/
└── parse/                ← NEW (slice)
    ├── elements/         ← NEW — L1 markdown tokenizer
    ├── spec/             ← NEW — capability spec parser
    ├── doc/              ← NEW — generic document parser
    ├── delta/            ← NEW — delta artifact parser
    └── step/             ← NEW — step artifact parser
```

### New capabilities

- `parse/elements` — line-by-line state machine producing an `Element` stream (Heading /
  Block / ListItem / BlockQuoteItem) with span tracking; infallible

- `parse/spec` — H1, summary, description, requirements, scenarios; GWT phase machine;
  test markers and backlinks; test-marker inheritance from requirement to scenario

- `parse/doc` — H1, summary, recursive section tree (used for capability docs, codex
  entries, `project.md`, proposals, designs)

- `parse/delta` — marker-prefixed entries (`= - ~ @ +`); canonical sort; rename-to
  extraction; per-marker child rules; **mandatory single space between marker and heading
  text** (parser tightened — see Impact)

- `parse/step` — named sections (Prerequisites / Context / Tasks / Outcomes); checkboxes;
  `@spec` / `@step` refs; subtasks

### Modified capabilities

None — this is the first slice; no caps exist yet.

### Out of scope

- Rendering (`render.rs`, `format.rs`) — separate slice

- Merge / delta application (`merge.rs`) — separate slice

- Audit / cross-artifact validation (`audit.rs`, `check.rs`) — separate slice

- The artifact data types themselves (`artifact/{spec,doc,delta,step}.rs`) — captured
  implicitly via parser output shape, not as standalone caps

- Format-side enforcement of the new whitespace rule — the parser rejects, but `ds format`
  is not updated to auto-insert the missing space

## Impact

```
┌─────────────────┐    ┌──────────────────┐    ┌──────────────┐
│ markdown source │───→│  parse_elements  │───→│ Layer 2      │
│   (.md files)   │    │   (infallible)   │    │ artifact     │
└─────────────────┘    └──────────────────┘    │ parsers      │
                                                └──────────────┘
                                                       │
                           ┌───────────────────────────┴────────┐
                           ▼              ▼              ▼      ▼
                         Spec         Document         Delta   Step
```

- **One small parser change**: `parse_marked_heading` will reject a marker that isn't
  followed by exactly one space, surfacing a new `ParseError::MarkerMissingSpace` variant.
  Affects `parse/delta.rs` only. Existing fixtures already use the canonical form, so no
  fixture updates needed.

- **No breaking change at the user-facing level**: every `.md` file in the repo today
  already uses `<marker> <text>` form. The tightening makes the implicit convention
  explicit.

- **Test additions**: 25 new tests across the 5 caps; no test infra changes (existing
  `parse_fixture(...).expect_err(...)` pattern works).

- **No migration**, **no new deps**, **no API change** outside of duckpond.

- Establishes the cap-tree convention `caps/<subsystem>/<area>/` for future backfill
  slices.
