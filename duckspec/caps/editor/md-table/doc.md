# Markdown table layout

Pure layout for GFM pipe tables over a line buffer: detect complete tables, size columns
to a character-cell pane with soft wrap inside cells, and map painted geometry back to
source positions without rewriting the buffer.

## What it produces

A layout pass takes source lines plus a pane width in monospace character cells and
returns zero or more table regions. Each region describes column widths, alignments, data
rows (header and body), per-cell soft-wrap fragments, and enough geometry to map between
visual positions and source positions on those lines.

```
source lines + pane_chars
        │
        ▼
  recognize complete GFM tables
        │
        ├── incomplete / broken → no region
        │
        ▼
  fit columns · soft-wrap cells · map
        │
        ▼
  TableLayout (regions)
```

Callers (for example a text editor’s hybrid layout) use the regions for visual rows and
hit-testing. The source line buffer is never modified.

## Recognition

A table region exists only when all of the following hold in order:

1. A header row of pipe-delimited cells
2. A separator row that defines per-column alignment (`---`, `:---`, `---:`, `:---:`)
3. One or more body rows with the **same** column count as the header

Anything else — missing separator, missing body, or a body row with a different column
count — yields no region for that span. Those lines stay ordinary text from the layout’s
point of view.

## Columns and wrapping

Widths are measured in character cells (not pixels).

```
| Step | Rule                                              |
|------|---------------------------------------------------|
| 1    | Natural width = max cell width per column         |
| 2    | Shrink widest columns until the table fits the    |
|      | pane, never below the minimum column width        |
| 3    | Soft-wrap each cell to its final column width     |
| 4    | If still too wide at all-mins, total width > pane |
```

A logical row’s **visual height** is the maximum number of soft-wrap fragments among its
cells. Empty or short cells simply contribute blank space on extra visual rows.

## Separator and display text

The separator line is alignment metadata only. It is not a data row and does not occupy
visual row space in the layout’s data-row list.

Display fragments are slices of **cell text**. Pipe delimiters (`|`) are not part of any
fragment. Callers that draw rules between columns use column edges from the layout, not
painted `|` characters.

## Source mapping

Every fragment knows which source line and which range of that line’s cell text it came
from. Mapping is bidirectional for positions inside cells:

- Fragment-local position → source position on that line within the cell
- Source position inside a cell → the fragment that covers it

Positions outside cells (for example on a `|` in the raw line) are not required to map
through the table geometry; callers treat those as ordinary line positions when no
fragment applies.

## What this is not

- Not a markdown parser for the rest of the document — only GFM pipe tables

- Not responsible for theme colors, zebra fills, or rule stroke styles

- Not a rewrite of clipboard or buffer text; copy stays the original markdown

- Not multi-line cells in the GFM sense (one source line per table row; soft wrap is
  visual only)
