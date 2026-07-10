//! Pure GFM pipe-table layout kernel.
//!
//! Detects complete GitHub-flavored markdown pipe tables in a line buffer,
//! parses cells and column alignments, and produces display geometry in
//! monospace character columns. Zero iced coupling — unit-tested pure
//! functions; the text editor multiplies by `cell_width` when painting.

use crate::widget::text_edit::Pos;
use std::ops::Range;

/// Minimum column width in characters when fitting to the pane.
/// Includes horizontal `CELL_PAD` on each side.
pub const MIN_COL_CHARS: usize = 8;

/// Gap in character cells drawn between adjacent columns (outside cell pads).
pub(crate) const COL_GAP: usize = 1;

/// Horizontal padding inside each cell, in character cells, on each side of
/// the display text. Keeps glyphs off the column rules.
pub(crate) const CELL_PAD: usize = 2;

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
    pub source_byte: Range<usize>,
    /// Trimmed cell text (display source for fragments).
    pub text: String,
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

/// Visual placement inside a laid-out table (table-local char coordinates).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TableVisualPos {
    pub region_idx: usize,
    pub visual_row_in_region: usize,
    pub char_col: usize,
}

/// Build table layout for `lines` given a pane width in monospace character
/// cells. Returns empty regions when nothing parses as a complete GFM table.
pub fn layout_tables(lines: &[String], pane_chars: usize) -> TableLayout {
    let mut regions = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        match try_parse_table(lines, i, pane_chars) {
            Some(region) => {
                let end = *region.source_lines.end();
                regions.push(region);
                i = end + 1;
            }
            None => i += 1,
        }
    }
    TableLayout { regions }
}

/// Map a source position to a visual placement inside a table region, if any.
pub fn source_to_visual(layout: &TableLayout, pos: Pos) -> Option<TableVisualPos> {
    for (region_idx, region) in layout.regions.iter().enumerate() {
        if !region.source_lines.contains(&pos.line) {
            continue;
        }
        let row_idx = region.rows.iter().position(|r| r.source_line == pos.line)?;
        let row = &region.rows[row_idx];
        let cell_idx = row
            .cells
            .iter()
            .position(|c| pos.col >= c.source_byte.start && pos.col <= c.source_byte.end)?;
        let cell = &row.cells[cell_idx];
        let byte_in_cell = pos.col.saturating_sub(cell.source_byte.start);
        let byte_in_cell = byte_in_cell.min(cell.text.len());
        // Snap to a char boundary within the cell text.
        let char_in_cell = cell.text[..byte_in_cell]
            .chars()
            .count()
            .min(cell.text.chars().count());

        let (frag_i, frag) = cell
            .fragments
            .iter()
            .enumerate()
            .find(|(_, f)| char_in_cell >= f.char_start && char_in_cell < f.char_end)
            .or_else(|| {
                // Caret at fragment end / past last → last fragment whose end
                // matches, else the final fragment.
                cell.fragments
                    .iter()
                    .enumerate()
                    .rev()
                    .find(|(_, f)| char_in_cell == f.char_end)
                    .or_else(|| {
                        cell.fragments
                            .last()
                            .map(|f| (cell.fragments.len() - 1, f))
                    })
            })?;

        let visual_row_before: usize = region.rows[..row_idx].iter().map(|r| r.height).sum();
        let visual_row_in_region = visual_row_before + frag_i;

        let frag_len = frag.char_end.saturating_sub(frag.char_start);
        let col_w = region.col_widths.get(cell_idx).copied().unwrap_or(0);
        let align = region.aligns.get(cell_idx).copied().unwrap_or(ColAlign::Left);
        let pad = align_pad(align, col_w, frag_len);
        let within = char_in_cell.saturating_sub(frag.char_start).min(frag_len);
        let char_col = col_origin(&region.col_widths, cell_idx) + pad + within;

        return Some(TableVisualPos {
            region_idx,
            visual_row_in_region,
            char_col,
        });
    }
    None
}

/// Map a point in table-local char coordinates (visual row within region,
/// char column from table left) back to a source `Pos`.
pub fn visual_to_source(
    layout: &TableLayout,
    region_idx: usize,
    visual_row_in_region: usize,
    char_col: usize,
) -> Option<Pos> {
    let region = layout.regions.get(region_idx)?;
    let mut row_start = 0usize;
    let mut row_idx = None;
    let mut frag_line = 0usize;
    for (i, row) in region.rows.iter().enumerate() {
        let row_end = row_start + row.height;
        if visual_row_in_region < row_end {
            row_idx = Some(i);
            frag_line = visual_row_in_region - row_start;
            break;
        }
        row_start = row_end;
    }
    let row_idx = row_idx?;
    let row = &region.rows[row_idx];

    let cell_idx = col_index_at(&region.col_widths, char_col)?;
    let cell = row.cells.get(cell_idx)?;
    let col_w = region.col_widths.get(cell_idx).copied().unwrap_or(0);
    let align = region.aligns.get(cell_idx).copied().unwrap_or(ColAlign::Left);
    let origin = col_origin(&region.col_widths, cell_idx);
    let local_x = char_col.saturating_sub(origin).min(col_w.saturating_sub(1).saturating_add(1));

    // Blank continuation rows (short cell in a tall logical row): caret at end.
    if frag_line >= cell.fragments.len() {
        return Some(Pos::new(row.source_line, cell.source_byte.end));
    }
    let frag = &cell.fragments[frag_line];
    let frag_len = frag.char_end.saturating_sub(frag.char_start);
    let pad = align_pad(align, col_w, frag_len);
    let within = if local_x <= pad {
        0
    } else {
        (local_x - pad).min(frag_len)
    };
    let char_in_cell = frag.char_start + within;
    let byte_in_cell = cell
        .text
        .char_indices()
        .nth(char_in_cell)
        .map(|(i, _)| i)
        .unwrap_or(cell.text.len());
    Some(Pos::new(
        row.source_line,
        cell.source_byte.start + byte_in_cell,
    ))
}

// ── Recognition & parsing ──────────────────────────────────────────────────

/// One pipe-delimited cell as it appears on a source line (trimmed text).
struct ParsedCell {
    /// Byte range of the trimmed cell text within the source line.
    source_byte: Range<usize>,
    /// Trimmed cell display text.
    text: String,
}

struct ParsedPipeRow {
    cells: Vec<ParsedCell>,
}

fn try_parse_table(lines: &[String], start: usize, pane_chars: usize) -> Option<TableRegion> {
    if start + 2 >= lines.len() {
        return None;
    }

    let header = parse_pipe_row(&lines[start])?;
    if header.cells.is_empty() {
        return None;
    }
    let col_count = header.cells.len();

    let (aligns, sep_cols) = parse_separator_row(&lines[start + 1])?;
    if sep_cols != col_count {
        return None;
    }

    let mut body_rows: Vec<(usize, ParsedPipeRow)> = Vec::new();
    let mut line_idx = start + 2;
    while line_idx < lines.len() {
        let line = &lines[line_idx];
        if line.trim().is_empty() {
            break;
        }
        let Some(row) = parse_pipe_row(line) else {
            break;
        };
        if row.cells.len() != col_count {
            // Mismatched body ends collection; if we already have bodies,
            // keep them. If not, fail the whole candidate.
            break;
        }
        body_rows.push((line_idx, row));
        line_idx += 1;
    }

    if body_rows.is_empty() {
        return None;
    }

    let last_body_line = body_rows.last().map(|(i, _)| *i).unwrap();

    // Natural column widths from header + body cell display widths, plus
    // horizontal cell padding so text isn't flush against the rules.
    let mut natural: Vec<usize> = vec![0; col_count];
    for (c, cell) in header.cells.iter().enumerate() {
        natural[c] = natural[c].max(cell.text.chars().count() + 2 * CELL_PAD);
    }
    for (_, row) in &body_rows {
        for (c, cell) in row.cells.iter().enumerate() {
            natural[c] = natural[c].max(cell.text.chars().count() + 2 * CELL_PAD);
        }
    }

    let col_widths = fit_col_widths(&natural, pane_chars);
    let total_width_chars = total_width(&col_widths);

    let mut rows = Vec::with_capacity(1 + body_rows.len());
    rows.push(build_table_row(start, RowRole::Header, &header, &col_widths));
    for (zebra_i, (src_line, parsed)) in body_rows.iter().enumerate() {
        rows.push(build_table_row(
            *src_line,
            RowRole::Body {
                zebra: zebra_i % 2 == 1,
            },
            parsed,
            &col_widths,
        ));
    }

    Some(TableRegion {
        source_lines: start..=last_body_line,
        col_widths,
        aligns,
        rows,
        total_width_chars,
    })
}

fn total_width(col_widths: &[usize]) -> usize {
    if col_widths.is_empty() {
        return 0;
    }
    col_widths.iter().sum::<usize>() + COL_GAP.saturating_mul(col_widths.len().saturating_sub(1))
}

/// Shrink widest columns first until the table fits `pane_chars`, never below
/// `MIN_COL_CHARS`. If still too wide at all-mins, leave mins (overflow).
fn fit_col_widths(natural: &[usize], pane_chars: usize) -> Vec<usize> {
    let mut widths = natural.to_vec();
    if widths.is_empty() {
        return widths;
    }
    // pane_chars 0 → force all-mins path via the same loop.
    loop {
        let tw = total_width(&widths);
        if tw <= pane_chars {
            break;
        }
        let Some(widest_i) = widths
            .iter()
            .enumerate()
            .filter(|(_, w)| **w > MIN_COL_CHARS)
            .max_by_key(|(_, w)| *w)
            .map(|(i, _)| i)
        else {
            break;
        };
        widths[widest_i] -= 1;
    }
    widths
}

/// Character-column origin of column `col` from the table's left edge.
pub(crate) fn col_origin(col_widths: &[usize], col: usize) -> usize {
    let col = col.min(col_widths.len());
    col_widths[..col].iter().sum::<usize>() + COL_GAP.saturating_mul(col)
}

/// Column index whose horizontal band contains `char_col` (table-local).
fn col_index_at(col_widths: &[usize], char_col: usize) -> Option<usize> {
    if col_widths.is_empty() {
        return None;
    }
    let mut x = 0usize;
    for (i, w) in col_widths.iter().enumerate() {
        if i + 1 == col_widths.len() {
            // Last column is open-ended so overflow x still maps here.
            return Some(i);
        }
        let band_end = x + *w + COL_GAP;
        if char_col < band_end {
            return Some(i);
        }
        x = band_end;
    }
    Some(col_widths.len() - 1)
}

/// Inner content width of a column (total width minus left/right cell pad).
pub(crate) fn content_width(col_width: usize) -> usize {
    col_width.saturating_sub(2 * CELL_PAD).max(1)
}

/// Leading pad inside a column for a fragment of length `frag_len`.
/// Includes left `CELL_PAD`, then alignment within the inner content band.
pub(crate) fn align_pad(align: ColAlign, col_width: usize, frag_len: usize) -> usize {
    let inner = content_width(col_width);
    let frag_len = frag_len.min(inner);
    let within = match align {
        ColAlign::Left => 0,
        ColAlign::Right => inner.saturating_sub(frag_len),
        ColAlign::Center => inner.saturating_sub(frag_len) / 2,
    };
    CELL_PAD + within
}

fn build_table_row(
    source_line: usize,
    role: RowRole,
    parsed: &ParsedPipeRow,
    col_widths: &[usize],
) -> TableRow {
    let cells: Vec<TableCell> = parsed
        .cells
        .iter()
        .enumerate()
        .map(|(c, cell)| {
            let width = col_widths.get(c).copied().unwrap_or(MIN_COL_CHARS);
            // Wrap to the inner content band so soft-wrap respects side padding.
            let fragments = soft_wrap_cell(&cell.text, content_width(width));
            TableCell {
                source_byte: cell.source_byte.clone(),
                text: cell.text.clone(),
                fragments,
            }
        })
        .collect();
    let height = cells.iter().map(|c| c.fragments.len().max(1)).max().unwrap_or(1);
    TableRow {
        source_line,
        role,
        cells,
        height,
    }
}

/// Soft-wrap `text` to rows of at most `max_chars`, preferring breaks after
/// spaces. Returns the char offset where each visual row starts (always
/// includes `0`). When `max_chars == 0`, returns `vec![0]` only.
///
/// Shared by table cell fragments and prose `wrap_line_starts` so break
/// semantics stay identical.
pub(crate) fn soft_wrap_starts(text: &str, max_chars: usize) -> Vec<usize> {
    if max_chars == 0 {
        return vec![0];
    }
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    if len <= max_chars {
        return vec![0];
    }

    let mut starts = vec![0usize];
    let mut pos = 0;
    while pos < len {
        let remaining = len - pos;
        if remaining <= max_chars {
            break;
        }
        let end = pos + max_chars;
        let break_at = (pos..end)
            .rev()
            .find(|&i| chars[i] == ' ')
            .map(|i| i + 1)
            .unwrap_or(end);
        let break_at = if break_at <= pos { end } else { break_at };
        starts.push(break_at);
        pos = break_at;
    }
    starts
}

/// Soft-wrap cell text to `max_chars`, preferring breaks after spaces.
fn soft_wrap_cell(text: &str, max_chars: usize) -> Vec<CellFragment> {
    let len = text.chars().count();
    if len == 0 {
        return vec![CellFragment {
            char_start: 0,
            char_end: 0,
        }];
    }
    let starts = soft_wrap_starts(text, max_chars.max(1));
    starts
        .iter()
        .enumerate()
        .map(|(i, &start)| {
            let end = starts.get(i + 1).copied().unwrap_or(len);
            CellFragment {
                char_start: start,
                char_end: end,
            }
        })
        .collect()
}

/// Parse a pipe-delimited data/header row. Returns `None` if the line is not a
/// plausible pipe row (no `|`, or empty after split).
fn parse_pipe_row(line: &str) -> Option<ParsedPipeRow> {
    if !line.contains('|') {
        return None;
    }
    // Separators are not data rows.
    if parse_separator_row(line).is_some() {
        return None;
    }

    let cells = split_pipe_cells(line)?;
    if cells.is_empty() {
        return None;
    }
    Some(ParsedPipeRow { cells })
}

/// Parse a GFM separator row (`| --- | :---: | ---: |`). Returns aligns and
/// column count, or `None` if the line is not a valid separator.
fn parse_separator_row(line: &str) -> Option<(Vec<ColAlign>, usize)> {
    if !line.contains('|') && !line.contains('-') {
        return None;
    }
    let raw_cells = split_raw_pipe_segments(line)?;
    if raw_cells.is_empty() {
        return None;
    }

    let mut aligns = Vec::with_capacity(raw_cells.len());
    for (_range, segment) in &raw_cells {
        let align = parse_align_marker(segment.trim())?;
        aligns.push(align);
    }
    let n = aligns.len();
    Some((aligns, n))
}

fn parse_align_marker(s: &str) -> Option<ColAlign> {
    if s.is_empty() {
        return None;
    }
    let bytes = s.as_bytes();
    let left = bytes[0] == b':';
    let right = bytes[bytes.len() - 1] == b':';
    let core = &s[if left { 1 } else { 0 }..s.len() - if right { 1 } else { 0 }];
    if core.is_empty() || !core.bytes().all(|b| b == b'-') {
        return None;
    }
    // Need at least one dash (GFM allows --- minimum commonly; accept 1+).
    match (left, right) {
        (true, true) => Some(ColAlign::Center),
        (false, true) => Some(ColAlign::Right),
        (true, false) | (false, false) => Some(ColAlign::Left),
    }
}

/// Split a pipe row into trimmed cells with source byte ranges for the trimmed
/// text. Leading/trailing empty segments from outer pipes are dropped.
fn split_pipe_cells(line: &str) -> Option<Vec<ParsedCell>> {
    let raw = split_raw_pipe_segments(line)?;
    let mut cells = Vec::with_capacity(raw.len());
    for (range, segment) in raw {
        let trimmed = segment.trim();
        // Locate trimmed slice inside `segment` to recover absolute bytes.
        let lead = segment.len() - segment.trim_start().len();
        let abs_start = range.start + lead;
        let abs_end = abs_start + trimmed.len();
        cells.push(ParsedCell {
            source_byte: abs_start..abs_end,
            text: trimmed.to_string(),
        });
    }
    Some(cells)
}

/// Split on `|`, dropping a leading empty segment (line starts with `|`) and a
/// trailing empty segment (line ends with `|`). Returns byte ranges into `line`
/// for each raw (untrimmed) segment.
fn split_raw_pipe_segments(line: &str) -> Option<Vec<(Range<usize>, &str)>> {
    if !line.contains('|') {
        return None;
    }

    let mut segments: Vec<(Range<usize>, &str)> = Vec::new();
    let mut start = 0;
    for (i, b) in line.bytes().enumerate() {
        if b == b'|' {
            segments.push((start..i, &line[start..i]));
            start = i + 1;
        }
    }
    segments.push((start..line.len(), &line[start..]));

    // Drop leading empty from opening `|`.
    if segments
        .first()
        .is_some_and(|(_, s)| s.is_empty())
    {
        segments.remove(0);
    }
    // Drop trailing empty from closing `|`.
    if segments
        .last()
        .is_some_and(|(_, s)| s.is_empty())
    {
        segments.pop();
    }

    if segments.is_empty() {
        return None;
    }
    Some(segments)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(xs: &[&str]) -> Vec<String> {
        xs.iter().map(|s| (*s).to_string()).collect()
    }

    fn fragment_text(cell_text: &str, frag: &CellFragment) -> String {
        cell_text
            .chars()
            .skip(frag.char_start)
            .take(frag.char_end.saturating_sub(frag.char_start))
            .collect()
    }

    fn cell_text_from_line(line: &str, cell: &TableCell) -> String {
        line[cell.source_byte.clone()].to_string()
    }

    /// @spec editor/md-table Table recognition: Complete header, separator, and body form a region
    #[test]
    fn complete_header_separator_body_forms_region() {
        let src = lines(&[
            "| Name | Value |",
            "| ---- | ----- |",
            "| a    | 1     |",
            "| b    | 2     |",
        ]);
        let layout = layout_tables(&src, 80);
        assert_eq!(layout.regions.len(), 1);
        let region = &layout.regions[0];
        assert_eq!(*region.source_lines.start(), 0);
        assert_eq!(*region.source_lines.end(), 3);
        // Header + two body rows; separator is not a data row.
        assert_eq!(region.rows.len(), 3);
        assert_eq!(region.rows[0].role, RowRole::Header);
        assert_eq!(region.rows[0].source_line, 0);
        assert_eq!(region.rows[1].source_line, 2);
        assert_eq!(region.rows[2].source_line, 3);
        assert_eq!(region.col_widths.len(), 2);
        assert!(region.rows.iter().all(|r| r.height >= 1));
        assert!(region.total_width_chars > 0);
    }

    /// @spec editor/md-table Table recognition: Missing separator or body yields no region
    #[test]
    fn missing_separator_or_body_yields_no_region() {
        // Header only — no separator, no body.
        let header_only = lines(&["| Name | Value |"]);
        assert!(layout_tables(&header_only, 80).regions.is_empty());

        // Header + separator, no body.
        let no_body = lines(&["| Name | Value |", "| ---- | ----- |"]);
        assert!(layout_tables(&no_body, 80).regions.is_empty());

        // Header + non-separator second line (not a valid table).
        let bad_sep = lines(&["| Name | Value |", "| not sep | either |", "| a | 1 |"]);
        assert!(layout_tables(&bad_sep, 80).regions.is_empty());
    }

    /// @spec editor/md-table Table recognition: Body column count mismatch yields no region
    #[test]
    fn body_column_count_mismatch_yields_no_region() {
        let src = lines(&[
            "| Name | Value |",
            "| ---- | ----- |",
            "| only-one-cell |",
        ]);
        assert!(layout_tables(&src, 80).regions.is_empty());
    }

    /// @spec editor/md-table Separator, aligns, and display text: Separator is not a data row and defines aligns
    #[test]
    fn separator_is_not_data_row_and_defines_aligns() {
        let src = lines(&[
            "| L | C | R |",
            "| :--- | :---: | ---: |",
            "| a | b | c |",
        ]);
        let layout = layout_tables(&src, 80);
        assert_eq!(layout.regions.len(), 1);
        let region = &layout.regions[0];

        // Separator line (index 1) must not appear as a data row.
        assert!(!region.rows.iter().any(|r| r.source_line == 1));
        assert_eq!(region.rows.len(), 2); // header + one body
        assert_eq!(
            region.aligns,
            vec![ColAlign::Left, ColAlign::Center, ColAlign::Right]
        );
        // Full GFM block still covers header..last body (separator included in range).
        assert_eq!(region.source_lines, 0..=2);
    }

    /// @spec editor/md-table Separator, aligns, and display text: Fragments omit pipe delimiters
    #[test]
    fn fragments_omit_pipe_delimiters() {
        let src = lines(&[
            "| Name | Value |",
            "| ---- | ----- |",
            "| alpha | beta |",
        ]);
        let layout = layout_tables(&src, 80);
        assert_eq!(layout.regions.len(), 1);
        let region = &layout.regions[0];

        for row in &region.rows {
            let line = &src[row.source_line];
            for cell in &row.cells {
                let text = cell_text_from_line(line, cell);
                assert!(
                    !text.contains('|'),
                    "cell source text must not include pipe: {text:?}"
                );
                for frag in &cell.fragments {
                    let ft = fragment_text(&text, frag);
                    assert_ne!(ft, "|", "fragment must not be a pipe delimiter");
                    assert!(
                        !ft.contains('|'),
                        "fragment text must not contain pipe: {ft:?}"
                    );
                    // Fragment is a contiguous slice of the cell's source text.
                    let expected: String = text
                        .chars()
                        .skip(frag.char_start)
                        .take(frag.char_end - frag.char_start)
                        .collect();
                    assert_eq!(ft, expected);
                }
            }
        }
    }

    #[test]
    fn incomplete_trailing_header_does_not_form_region() {
        // Prose plus a complete table, then an incomplete trailer.
        let src = lines(&[
            "intro",
            "| H |",
            "| - |",
            "| v |",
            "",
            "| alone |",
        ]);
        let layout = layout_tables(&src, 80);
        assert_eq!(layout.regions.len(), 1);
        assert_eq!(layout.regions[0].source_lines, 1..=3);
    }

    #[test]
    fn left_align_dash_only_separator() {
        let src = lines(&["| A | B |", "| --- | --- |", "| 1 | 2 |"]);
        let layout = layout_tables(&src, 80);
        assert_eq!(layout.regions.len(), 1);
        assert_eq!(
            layout.regions[0].aligns,
            vec![ColAlign::Left, ColAlign::Left]
        );
    }

    /// @spec editor/md-table Column fit and cell wrap: Short cells produce a total width within the pane
    #[test]
    fn short_cells_total_width_within_pane() {
        let src = lines(&["| A | B |", "| - | - |", "| x | y |"]);
        let pane = 80;
        let layout = layout_tables(&src, pane);
        assert_eq!(layout.regions.len(), 1);
        let region = &layout.regions[0];
        // Natural widths are tiny; total must fit the wide pane.
        assert!(
            region.total_width_chars <= pane,
            "total {} > pane {}",
            region.total_width_chars,
            pane
        );
    }

    /// @spec editor/md-table Column fit and cell wrap: A long cell soft-wraps within the pane
    #[test]
    fn long_cell_soft_wraps_within_pane() {
        // One long body cell; two columns still fit at/above MIN in a mid pane.
        let long = "word ".repeat(20); // plenty of soft-wrap opportunities
        let long = long.trim_end();
        let src = lines(&[
            "| H1 | H2 |",
            "| --- | --- |",
            &format!("| {long} | ok |"),
        ]);
        // Pane wide enough for 2 * MIN + gap, narrow enough to force wrap of long cell.
        let pane = MIN_COL_CHARS * 2 + COL_GAP + 4; // 21
        let layout = layout_tables(&src, pane);
        assert_eq!(layout.regions.len(), 1);
        let region = &layout.regions[0];
        assert!(
            region.total_width_chars <= pane,
            "total {} > pane {}",
            region.total_width_chars,
            pane
        );
        let body = &region.rows[1];
        let long_cell = &body.cells[0];
        assert!(
            long_cell.fragments.len() > 1,
            "expected soft-wrap, got {} fragments (col_w={:?})",
            long_cell.fragments.len(),
            region.col_widths
        );
        assert!(
            body.height > 1,
            "row height should exceed one, got {}",
            body.height
        );
    }

    /// @spec editor/md-table Column fit and cell wrap: Many minimum-width columns may exceed the pane
    #[test]
    fn many_min_width_columns_may_exceed_pane() {
        // 3 columns with long natural widths; at MIN each, total exceeds a small pane.
        let src = lines(&[
            "| AAAAAAAAAA | BBBBBBBBBB | CCCCCCCCCC |",
            "| ---------- | ---------- | ---------- |",
            "| aaaaaaaaaa | bbbbbbbbbb | cccccccccc |",
        ]);
        let pane = MIN_COL_CHARS * 2; // 16 < 3*8 + 2 gaps
        let layout = layout_tables(&src, pane);
        assert_eq!(layout.regions.len(), 1);
        let region = &layout.regions[0];
        assert!(
            region.total_width_chars > pane,
            "expected overflow: total {} <= pane {}",
            region.total_width_chars,
            pane
        );
        assert!(region.col_widths.iter().all(|&w| w == MIN_COL_CHARS));
    }

    /// @spec editor/md-table Source mapping: Fragment position maps into the cell’s source text
    #[test]
    fn fragment_position_maps_into_cell_source_text() {
        let src = lines(&[
            "| Name | Value |",
            "| ---- | ----- |",
            "| alpha | beta |",
        ]);
        let layout = layout_tables(&src, 80);
        assert_eq!(layout.regions.len(), 1);
        let region = &layout.regions[0];
        let row = &region.rows[1]; // body
        let cell = &row.cells[0];
        assert!(!cell.text.is_empty());
        assert!(!cell.fragments.is_empty());
        let frag = &cell.fragments[0];
        assert!(frag.char_end > frag.char_start);

        // Mid-fragment visual position → source.
        let visual_row_before: usize = region.rows[..1].iter().map(|r| r.height).sum();
        let pad = align_pad(
            region.aligns[0],
            region.col_widths[0],
            frag.char_end - frag.char_start,
        );
        let char_col = col_origin(&region.col_widths, 0) + pad + 1; // second char
        let pos = visual_to_source(&layout, 0, visual_row_before, char_col)
            .expect("visual_to_source");
        assert_eq!(pos.line, row.source_line);
        assert!(
            pos.col >= cell.source_byte.start && pos.col <= cell.source_byte.end,
            "pos.col {} outside cell {:?}",
            pos.col,
            cell.source_byte
        );
    }

    /// @spec editor/md-table Source mapping: Source position in a cell maps to a fragment of that cell
    #[test]
    fn source_position_in_cell_maps_to_fragment_of_that_cell() {
        let src = lines(&[
            "| Name | Value |",
            "| ---- | ----- |",
            "| alpha | beta |",
        ]);
        let layout = layout_tables(&src, 80);
        assert_eq!(layout.regions.len(), 1);
        let region = &layout.regions[0];
        let row = &region.rows[1];
        let cell = &row.cells[0];
        assert!(!cell.text.is_empty());

        // Source position on the second character of the cell.
        let byte_off = cell.text.chars().next().map(|c| c.len_utf8()).unwrap_or(0);
        let pos = Pos::new(row.source_line, cell.source_byte.start + byte_off);
        let visual = source_to_visual(&layout, pos).expect("source_to_visual");
        assert_eq!(visual.region_idx, 0);

        // Round-trip identifies the same cell: mapped source stays in range,
        // and visual row falls within this row's band.
        let visual_row_before: usize = region.rows[..1].iter().map(|r| r.height).sum();
        assert!(
            visual.visual_row_in_region >= visual_row_before
                && visual.visual_row_in_region < visual_row_before + row.height
        );
        let back = visual_to_source(
            &layout,
            visual.region_idx,
            visual.visual_row_in_region,
            visual.char_col,
        )
        .expect("round-trip");
        assert_eq!(back.line, row.source_line);
        assert!(
            back.col >= cell.source_byte.start && back.col <= cell.source_byte.end,
            "round-trip col {} outside cell {:?}",
            back.col,
            cell.source_byte
        );
        // Lands on a display fragment of this cell (char offset inside some fragment).
        let char_in = cell.text[..back.col.saturating_sub(cell.source_byte.start).min(cell.text.len())]
            .chars()
            .count();
        // Use the original source char offset for fragment membership.
        let char_from_src = cell.text[..byte_off].chars().count();
        assert!(
            cell.fragments
                .iter()
                .any(|f| char_from_src >= f.char_start && char_from_src <= f.char_end),
            "char {char_from_src} not in any fragment of the cell; char_in={char_in}"
        );
    }
}
