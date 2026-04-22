//! Detect GFM-style tables in paragraph content.
//!
//! A GFM table has a header row containing at least one `|`, a delimiter row
//! whose cells are dashes (optionally with `:` for alignment), and zero or
//! more body rows each containing at least one `|`. The whole paragraph must
//! be the table — leading or trailing prose disqualifies it.

/// Returns `true` when `content` is a complete GFM table.
pub fn is_gfm_table(content: &str) -> bool {
    let mut lines = content.lines();
    let Some(header) = lines.next() else {
        return false;
    };
    if !header.contains('|') {
        return false;
    }
    let Some(delimiter) = lines.next() else {
        return false;
    };
    if !is_delimiter_row(delimiter) {
        return false;
    }
    lines.all(|line| line.contains('|'))
}

/// Returns `true` when `line` is a valid GFM table delimiter row, e.g.
/// `| --- | :---: |` or `---|:--`.
fn is_delimiter_row(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() || !trimmed.contains('-') {
        return false;
    }
    let inner = trimmed
        .strip_prefix('|')
        .unwrap_or(trimmed)
        .strip_suffix('|')
        .unwrap_or_else(|| trimmed.strip_prefix('|').unwrap_or(trimmed));
    let cells: Vec<&str> = inner.split('|').collect();
    if cells.is_empty() {
        return false;
    }
    cells.iter().all(|cell| is_delimiter_cell(cell))
}

fn is_delimiter_cell(cell: &str) -> bool {
    let cell = cell.trim();
    if cell.is_empty() {
        return false;
    }
    let bytes = cell.as_bytes();
    let mut i = 0;
    if bytes[i] == b':' {
        i += 1;
    }
    let dash_start = i;
    while i < bytes.len() && bytes[i] == b'-' {
        i += 1;
    }
    if i == dash_start {
        return false;
    }
    if i < bytes.len() && bytes[i] == b':' {
        i += 1;
    }
    i == bytes.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_table() {
        let t = "| h1 | h2 |\n|----|----|\n| a  | b  |";
        assert!(is_gfm_table(t));
    }

    #[test]
    fn table_without_outer_pipes() {
        let t = "h1 | h2\n---|---\na  | b";
        assert!(is_gfm_table(t));
    }

    #[test]
    fn table_with_alignment() {
        let t = "| left | center | right |\n| :--- | :----: | ----: |\n| a    | b      | c     |";
        assert!(is_gfm_table(t));
    }

    #[test]
    fn header_only_no_delimiter() {
        let t = "| h1 | h2 |";
        assert!(!is_gfm_table(t));
    }

    #[test]
    fn header_and_prose_no_delimiter() {
        let t = "| h1 | h2 |\nthis is prose";
        assert!(!is_gfm_table(t));
    }

    #[test]
    fn prose_then_table_is_not_a_table() {
        let t = "intro line\n| h | h |\n|---|---|\n| a | b |";
        assert!(!is_gfm_table(t));
    }

    #[test]
    fn body_row_without_pipe_disqualifies() {
        let t = "| h1 | h2 |\n|----|----|\n| a  | b  |\nstray prose";
        assert!(!is_gfm_table(t));
    }

    #[test]
    fn delimiter_row_with_invalid_cells() {
        let t = "| h1 | h2 |\n| abc | def |\n| a  | b  |";
        assert!(!is_gfm_table(t));
    }

    #[test]
    fn empty_input() {
        assert!(!is_gfm_table(""));
    }

    #[test]
    fn single_line_with_pipe_is_not_a_table() {
        assert!(!is_gfm_table("a | b"));
    }

    #[test]
    fn table_with_only_header_and_delimiter() {
        let t = "| h1 | h2 |\n|----|----|";
        assert!(is_gfm_table(t));
    }

    #[test]
    fn delimiter_cell_minimum_one_dash() {
        let t = "| a |\n| - |";
        assert!(is_gfm_table(t));
    }

    #[test]
    fn delimiter_with_only_colon_no_dash() {
        let t = "| a |\n| : |";
        assert!(!is_gfm_table(t));
    }

    #[test]
    fn delimiter_with_extra_chars_in_cell_is_invalid() {
        let t = "| a |\n| -x- |";
        assert!(!is_gfm_table(t));
    }
}
