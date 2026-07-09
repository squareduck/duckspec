# Render Markdown Tables in TextEdit — Design

A pure GFM table layout kernel feeds a hybrid `TextEdit` layout (opt-in via `md_tables`)
so chat-style editors can paint fit-to-pane grids with soft cell wrap while selection and
copy stay on the original markdown source.

## Approach

```
EditorState.lines  (source markdown — never rewritten)
        │
        ▼
┌────────────────────────────────────────────────────┐
│  widget::md_table          pure · unit-tested      │
│  detect regions → parse cells/aligns → fit widths  │
│  → soft-wrap fragments → TableLayout + source map  │
└────────────────────────────┬───────────────────────┘
                             │ only when md_tables
                             ▼
┌────────────────────────────────────────────────────┐
│  text_edit::EditorLayout   hybrid visual rows      │
│  per logical line:                                 │
│    covered by TableLayout → table visual band      │
│    else + word_wrap       → WrapLayout (today)     │
│    else                   → 1:1 line               │
│  total_visual_rows · content_width · hit-test      │
└──────────────┬─────────────────────┬───────────────┘
               ▼                     ▼
        draw fragments         pixel → Pos
        + chrome (impl)        selection_text = source
```

**Strategy.** Keep `EditorState.lines` as the sole buffer. All “pretty table” behavior is
a view transform: detect GFM pipe tables, compute column geometry that prefers fitting the
pane (shrink + cell wrap), and only exceed the pane when many columns hit a minimum width
— then widen content and enable `scroll_x`. Chat turns `.md_tables(true)` on; file tabs
leave it off so ordinary markdown files keep today’s line wrap only.

Chrome (header weight, zebra, rules, hidden pipes) is paint on top of layout *roles*; the
cap and the pure kernel care about geometry and source mapping, not colors.

## Pure kernel — `widget/md_table.rs`

New module next to `autoscroll`: zero iced coupling, char-grid metrics only (widths in
character columns; the renderer multiplies by `cell_width`).

```
lines + pane_chars
        │
        ▼
  detect consecutive GFM rows
  (header + separator + ≥1 data)
        │
        ├── incomplete / uneven / no separator → no region
        │
        ▼
  parse cells, aligns from :--- :--- :---
  separator is metadata only (not a data row)
        │
        ▼
  natural col widths → shrink longest-first to MIN
  → soft-wrap each cell to col width
  → row_height = max(fragments across cells)
        │
        ▼
  TableLayout { regions, visual geometry, maps }
```

```rust
/// Minimum column width in characters when fitting to the pane.
pub const MIN_COL_CHARS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColAlign {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowRole {
    Header,
    Body { zebra: bool },
}

/// One visual fragment of a cell after soft wrap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellFragment {
    /// Char offset into the cell's source text (not the full line).
    pub char_start: usize,
    /// Exclusive end.
    pub char_end: usize,
}

#[derive(Debug, Clone)]
pub struct TableCell {
    /// Byte range of this cell's text inside the source line (excluding pipes
    /// and surrounding padding spaces the kernel normalizes).
    pub source_byte: std::ops::Range<usize>,
    pub fragments: Vec<CellFragment>,
}

#[derive(Debug, Clone)]
pub struct TableRow {
    pub source_line: usize,
    pub role: RowRole,
    pub cells: Vec<TableCell>,
    /// Visual rows this logical table row occupies.
    pub height: usize,
}

#[derive(Debug, Clone)]
pub struct TableRegion {
    /// Inclusive source line range of the full GFM block (header..last data),
    /// including the separator line which has no `TableRow`.
    pub source_lines: std::ops::RangeInclusive<usize>,
    pub col_widths: Vec<usize>,
    pub aligns: Vec<ColAlign>,
    pub rows: Vec<TableRow>,
    /// Sum of col_widths + inter-column gaps (chars), used for content width.
    pub total_width_chars: usize,
}

#[derive(Debug, Clone, Default)]
pub struct TableLayout {
    pub regions: Vec<TableRegion>,
}

/// Build table layout for `lines` given a pane width in monospace character
/// cells. Returns empty regions when nothing parses as a complete GFM table.
pub fn layout_tables(lines: &[String], pane_chars: usize) -> TableLayout {
    todo!()
}

/// Map a source position to a visual placement inside a table region, if any.
pub fn source_to_visual(
    layout: &TableLayout,
    pos: crate::widget::text_edit::Pos,
) -> Option<TableVisualPos> {
    todo!()
}

/// Map a point in table-local char coordinates (visual row within region,
/// char column from table left) back to a source `Pos`.
pub fn visual_to_source(
    layout: &TableLayout,
    region_idx: usize,
    visual_row_in_region: usize,
    char_col: usize,
) -> Option<crate::widget::text_edit::Pos> {
    todo!()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TableVisualPos {
    pub region_idx: usize,
    pub visual_row_in_region: usize,
    pub char_col: usize,
}
```

**Detection rules (kernel contract):**

- A table starts at a pipe row immediately followed by a separator row matching
  `|? --- | --- | …` with optional `:---` / `---:` / `:---:` aligns.

- One or more body pipe rows with the same column count follow.

- Blank line or non-pipe row ends the region.

- Incomplete (header only, missing body) or inconsistent column counts → not a region;
  those lines stay on ordinary wrap.

**Display text:** fragments cover cell text only. `|` delimiters and the separator line
produce no painted text (paint draws rules from column edges).

**Fit algorithm:**

1. Natural width = max over header+body of cell display width (chars).

2. While `sum(widths) + gaps > pane_chars` and some width > `MIN_COL_CHARS`, decrement the
   widest column.

3. Soft-wrap each cell to its final width (space-preferring, same spirit as
   `wrap_line_starts`).

4. If still wider than pane at all-mins → keep mins; `total_width_chars` may exceed
   `pane_chars` (overflow path for hybrid layout).

## Hybrid layout — `text_edit/render.rs`

Today: `word_wrap` toggles a global `WrapLayout` and forces `scroll_x = 0`. Tables need
region-aware geometry and occasional horizontal overflow.

```rust
pub struct TextEdit<'a, M> {
    // …
    word_wrap: bool,
    /// When true (and typically with word_wrap), GFM tables use TableLayout.
    md_tables: bool,
}

impl<'a, M> TextEdit<'a, M> {
    pub fn md_tables(mut self, enabled: bool) -> Self {
        self.md_tables = enabled;
        self
    }
}

/// Unified visual layout for one frame.
struct EditorLayout {
    /// Total visual rows across prose + tables.
    total_visual_rows: usize,
    /// Max content width in chars (pane for wrapped prose; may be larger when
    /// a table overflows at MIN_COL).
    content_width_chars: usize,
    /// Optional table kernel output (empty when md_tables is off).
    tables: TableLayout,
    /// Prose wrap for non-table lines (None when word_wrap is false).
    wrap: Option<WrapLayout>,
    /// For each logical line: how it participates in visual space.
    line_kind: Vec<LineLayoutKind>,
}

enum LineLayoutKind {
    /// Ordinary line: 1 visual row, or wrap sub-rows via WrapLayout.
    Prose,
    /// Separator line inside a table: 0 visual rows (aligns only).
    TableSeparator { region: usize },
    /// Header/body row: height from TableRow.
    TableRow { region: usize, row: usize },
}
```

**Build order per frame** (layout + draw + hit-test share one compute):

```
pane_chars = content_w_px / cell_w
tables = if md_tables { layout_tables(lines, pane_chars) } else { empty }
for each logical line:
  if in table region as separator → 0 visual rows
  if in table region as data/header → TableRow.height visual rows
  else if word_wrap → wrap_line_starts(…, pane_chars)
  else → 1 row
content_width_chars = max(pane_chars, max region.total_width_chars)
scroll_x allowed when content_width_chars > pane_chars
```

`agent_chat::view_block` (and tool bodies that use the same path) pass `.md_tables(true)`.
File tabs / search slices leave the default `false`.

## Hit-test and selection

`selection_text` / cmd-a already slice `EditorState.lines` by `Pos`. Correctness is
entirely in mapping paint coordinates ↔ `Pos`.

```
pixel_to_pos:
  vrow from y
  if vrow in a table band:
    region-local row + x → visual_to_source → Pos
  else:
    existing wrap / 1:1 path

selection paint:
  for each selected source range intersecting a table row:
    cover the CellFragments that overlap the range (per-cell quads)
  prose ranges: existing line/sub-row highlight
```

Arrow up/down when the cursor is on a table row walk **visual** rows via the same map
(extend `visual_up_target` / `visual_down_target` to consult `EditorLayout`, not only
`WrapLayout`).

## Chrome paint (implementation only)

Not part of `editor/md-table` requirements. Renderer uses layout roles:

```
for each table visual band:
  1. fill row bg from RowRole (header vs zebra)
  2. selection quads (fragment-clipped)
  3. cell fragment text (aligned per ColAlign; no pipe glyphs)
  4. vertical/horizontal rules at column edges and row bands
```

Theme tokens (names illustrative): header bg, even/odd body bg, rule color. Pipes and the
separator source line are never drawn as text.

## Call sites

```
agent_chat::view_block / view_tool_block
  TextEdit::new(…)
    .word_wrap(true)
    .md_tables(true)    // ← enable
    .read_only(true)
    …

file tabs / queue input (optional later)
  default md_tables(false)
```

No `chat_store` or persistence changes. No `duckpond` changes.

## Decisions

- **Pure `widget/md_table` module** — mirrors `autoscroll`: unit-tested kernel, thin iced
  shell. Alternatives: fold into `render.rs` (rejected: harder to test, couples geometry
  to widget state).

- **Opt-in `md_tables` flag** — chat enables; file editors do not. Alternatives: always-on
  whenever `word_wrap` (rejected: pipe-heavy prose and raw markdown editing would
  surprise; file tabs should stay line-faithful).

- **Fit then overflow** — shrink to `MIN_COL_CHARS` + soft cell wrap; if still wider, set
  `content_width_chars` above the pane and use editor `scroll_x`. Alternatives: crush
  below min (unreadable); ellipsis (loses agent data); nested table-only scroll (complex
  hit-test, fights outer chat scroll).

- **Separator is metadata only** — no visual row for `|---|---|`. Aligns come from it;
  paint uses rules instead.

- **Display fragments omit pipes** — source still has `|`; copy is unchanged.
  Alternatives: keep faint pipes (rejected: noisier next to drawn rules).

- **Char-grid metrics** — all widths in characters × existing monospace `cell_width`.
  Alternatives: proportional measure (rejected: proposal out of scope; breaks simple
  hit-test math).

- **Streaming** — incomplete tables stay prose until a full GFM block parses.
  Alternatives: speculative partial grids (rejected: layout thrash, wrong column counts).

## Risks

- **Hit-test vs paint drift** → single `EditorLayout` computed once per frame and shared
  by draw, `pixel_to_pos`, and visual caret motion; unit-test the kernel maps
  exhaustively.

- **False-positive tables** → require header + valid separator + ≥1 body with matching
  column counts; otherwise plain wrap.

- **Layout jump when a streaming table completes** → accepted (proposal: eventual
  correctness); no intermediate half-grids.

- **Wide tables + chat outer scroll** → horizontal pan is editor-local `scroll_x`;
  vertical remains outer chat scroll / `fit_content` as today.

- **Syntax highlight spans on table lines** → remap or suppress pipe-line syntect colors
  when drawing fragments (prefer plain/default text for cell body to avoid misaligned span
  offsets); detail at implement time.

## Open questions

None — flag and overflow UX resolved in design review.
