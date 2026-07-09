# Render Markdown Tables in TextEdit

Detect GFM pipe tables inside duckboard’s shared `TextEdit` and lay them out as
fit-to-pane grids with soft cell wrap, while keeping the source buffer and selection/copy
positions on the original markdown.

## Motivation

Agent chat routinely emits comparison tables. Today those rows are monospaced pipe text
with ordinary word-wrap, so columns break mid-row and the content is hard to scan. The fix
belongs in the one editor that paints message bodies — not in a second widget tree that
would break cmd-a / cmd-c across a message.

Why now: harness work is landing more structured agent output, and the readability gap is
pure view-layer debt with a clean pure-function core.

## Scope

```
caps/
├── chat/persistence/     (unchanged)
├── editor/               ← NEW namespace
│   └── md-table/         ← NEW  (layout + source map only)
└── ...
```

### New capabilities

- `editor/md-table` — pure GFM table layout for TextEdit: detect table regions in a line
  buffer; parse cells and column aligns; fit column widths to a pane width with soft wrap
  inside cells; build a visual geometry (row heights, fragment ranges); map
  pointer/selection positions back to source `Pos`; treat the separator row as align
  metadata only (not a data row); fall incomplete or broken tables back to ordinary line
  layout until they parse cleanly. Display fragments are cell text only (no `|`
  delimiters).

### Modified capabilities

- None

### Out of scope

- Paint chrome details (header weight, zebra fills, rule thickness/colors) —
  implementation on top of layout roles, not cap requirements

- Hard multi-line cells, HTML tables, or `<br>` forced breaks

- Rewriting clipboard text (copy stays source markdown)

- TSV/CSV “copy as table” actions

- Streaming-perfect intermediate layouts (eventual correctness is enough)

- Proportional fonts or non-monospace metrics

- duckpond, ds, or duckchat changes

## Impact

```
EditorState.lines (unchanged markdown)
        │
        ▼
  md-table layout kernel   ← NEW, unit-tested
   detect · widths · wrap · map
        │
        ▼
  TextEdit hybrid layout
   prose → WrapLayout (today)
   table → TableLayout roles
        │
        ▼
  paint chrome (impl) · hit-test → source Pos → cmd-c
```

- **duckboard only** — pure layout helper plus `widget/text_edit` wiring; chat keeps one
  editor block per content chunk.

- No on-disk format change, no new dependencies expected.

- Applies anywhere `TextEdit` lays out multi-line content that may contain GFM tables
  (chat bodies, tool output, etc.).
