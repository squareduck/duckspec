//! Word-wrap prose at a target column width while preserving inline-atomic
//! constructs (code spans, links, images, autolinks) as indivisible units.

/// Reflow `input` to lines no wider than `width` characters, breaking only at
/// whitespace between chunks. Atomic inline constructs are never split across
/// lines; an atom wider than `width` is emitted on its own line.
pub fn reflow(input: &str, width: usize) -> String {
    let chunks = tokenize(input);
    let mut out = String::new();
    let mut col = 0usize;
    for chunk in &chunks {
        let chunk_w = display_width(chunk);
        if col == 0 {
            out.push_str(chunk);
            col = chunk_w;
        } else if col + 1 + chunk_w <= width {
            out.push(' ');
            out.push_str(chunk);
            col += 1 + chunk_w;
        } else {
            out.push('\n');
            out.push_str(chunk);
            col = chunk_w;
        }
    }
    out
}

/// Split `input` into whitespace-separated chunks, consuming atomic inline
/// constructs (code spans, links, images, autolinks) whole — even when they
/// contain internal whitespace.
pub fn tokenize(input: &str) -> Vec<String> {
    let bytes = input.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        while i < bytes.len() && is_ws(bytes[i]) {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let start = i;
        while i < bytes.len() && !is_ws(bytes[i]) {
            match bytes[i] {
                b'`' => {
                    if let Some(end) = find_code_span_end(bytes, i) {
                        i = end;
                    } else {
                        i += 1;
                    }
                }
                b'[' => {
                    if let Some(end) = find_link_end(bytes, i) {
                        i = end;
                    } else {
                        i += 1;
                    }
                }
                b'!' if i + 1 < bytes.len() && bytes[i + 1] == b'[' => {
                    if let Some(end) = find_link_end(bytes, i + 1) {
                        i = end;
                    } else {
                        i += 1;
                    }
                }
                b'<' => {
                    if let Some(end) = find_autolink_end(bytes, i) {
                        i = end;
                    } else {
                        i += 1;
                    }
                }
                _ => {
                    i += 1;
                }
            }
        }
        tokens.push(std::str::from_utf8(&bytes[start..i]).unwrap().to_string());
    }
    tokens
}

fn is_ws(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r')
}

fn display_width(s: &str) -> usize {
    s.chars().count()
}

/// Find the end of a code span starting at `start` (which must point at a
/// backtick). Returns the index just past the closing backtick run. A code
/// span closes on a run of backticks equal in length to the opening run.
fn find_code_span_end(bytes: &[u8], start: usize) -> Option<usize> {
    let mut open_len = 0;
    let mut i = start;
    while i < bytes.len() && bytes[i] == b'`' {
        open_len += 1;
        i += 1;
    }
    while i < bytes.len() {
        if bytes[i] == b'`' {
            let mut close_len = 0;
            while i < bytes.len() && bytes[i] == b'`' {
                close_len += 1;
                i += 1;
            }
            if close_len == open_len {
                return Some(i);
            }
        } else {
            i += 1;
        }
    }
    None
}

/// Find the end of a link `[text](url)` starting at `start` (which must point
/// at `[`). Returns the index just past the closing paren.
fn find_link_end(bytes: &[u8], start: usize) -> Option<usize> {
    debug_assert_eq!(bytes.get(start), Some(&b'['));
    let mut i = start + 1;
    let mut depth = 1usize;
    while i < bytes.len() && depth > 0 {
        match bytes[i] {
            b'\\' if i + 1 < bytes.len() => i += 2,
            b'[' => {
                depth += 1;
                i += 1;
            }
            b']' => {
                depth -= 1;
                i += 1;
            }
            _ => i += 1,
        }
    }
    if depth != 0 || i >= bytes.len() || bytes[i] != b'(' {
        return None;
    }
    i += 1;
    let mut pdepth = 1usize;
    while i < bytes.len() && pdepth > 0 {
        match bytes[i] {
            b'\\' if i + 1 < bytes.len() => i += 2,
            b'(' => {
                pdepth += 1;
                i += 1;
            }
            b')' => {
                pdepth -= 1;
                i += 1;
            }
            _ => i += 1,
        }
    }
    if pdepth != 0 {
        return None;
    }
    Some(i)
}

/// Find the end of an autolink `<scheme:...>` starting at `start` (which must
/// point at `<`). Returns the index just past `>`. Requires at least one `:`
/// inside to avoid matching HTML-like `<tag>`.
fn find_autolink_end(bytes: &[u8], start: usize) -> Option<usize> {
    debug_assert_eq!(bytes.get(start), Some(&b'<'));
    let mut i = start + 1;
    let mut has_colon = false;
    while i < bytes.len() {
        match bytes[i] {
            b'>' => {
                if has_colon && i > start + 1 {
                    return Some(i + 1);
                }
                return None;
            }
            b' ' | b'\t' | b'\n' | b'\r' | b'<' => return None,
            b':' => {
                has_colon = true;
                i += 1;
            }
            _ => i += 1,
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input() {
        assert_eq!(reflow("", 80), "");
    }

    #[test]
    fn single_short_line() {
        assert_eq!(reflow("hello world", 80), "hello world");
    }

    #[test]
    fn collapses_runs_of_whitespace() {
        assert_eq!(reflow("a   b\t\tc\n\nd", 80), "a b c d");
    }

    #[test]
    fn wraps_at_width() {
        let input = "one two three four five";
        // width 10: "one two" (7) | "three four" (10) | "five" (4)
        assert_eq!(reflow(input, 10), "one two\nthree four\nfive");
    }

    #[test]
    fn long_word_gets_own_line() {
        let input = "short superduperlongword short";
        assert_eq!(reflow(input, 10), "short\nsuperduperlongword\nshort");
    }

    #[test]
    fn keeps_inline_code_atomic() {
        // Code span contains a space; must stay on one chunk.
        let input = "pre `hello world` post";
        let out = reflow(input, 12);
        // Chunks: ["pre", "`hello world`", "post"]
        // width 12: "pre" (3) + " `hello world`" (14) → overflow, new line.
        // Line 2: "`hello world`" (13) > 12 → own line.
        // Line 3: "post".
        assert_eq!(out, "pre\n`hello world`\npost");
    }

    #[test]
    fn inline_code_mid_word_stays_with_word() {
        let input = "foo`bar baz`qux end";
        let chunks = tokenize(input);
        assert_eq!(chunks, vec!["foo`bar baz`qux", "end"]);
    }

    #[test]
    fn unterminated_code_span_is_literal() {
        let input = "foo `bar baz";
        let chunks = tokenize(input);
        assert_eq!(chunks, vec!["foo", "`bar", "baz"]);
    }

    #[test]
    fn double_backtick_code_span() {
        let input = "x ``a ` b`` y";
        let chunks = tokenize(input);
        assert_eq!(chunks, vec!["x", "``a ` b``", "y"]);
    }

    #[test]
    fn keeps_link_atomic() {
        let input = "see [click here](https://example.com) now";
        let chunks = tokenize(input);
        assert_eq!(
            chunks,
            vec!["see", "[click here](https://example.com)", "now"]
        );
    }

    #[test]
    fn keeps_image_atomic() {
        let input = "![alt text](path/to/img.png) caption";
        let chunks = tokenize(input);
        assert_eq!(
            chunks,
            vec!["![alt text](path/to/img.png)", "caption"]
        );
    }

    #[test]
    fn autolink_atomic() {
        let input = "visit <https://example.com/long> soon";
        let chunks = tokenize(input);
        assert_eq!(
            chunks,
            vec!["visit", "<https://example.com/long>", "soon"]
        );
    }

    #[test]
    fn plain_angle_tag_not_autolink() {
        let input = "use <div> here";
        let chunks = tokenize(input);
        assert_eq!(chunks, vec!["use", "<div>", "here"]);
    }

    #[test]
    fn unterminated_link_is_literal() {
        let input = "foo [bar baz qux";
        let chunks = tokenize(input);
        assert_eq!(chunks, vec!["foo", "[bar", "baz", "qux"]);
    }

    #[test]
    fn link_with_escaped_bracket() {
        let input = r"see [a \] b](url) end";
        let chunks = tokenize(input);
        assert_eq!(chunks, vec!["see", r"[a \] b](url)", "end"]);
    }

    #[test]
    fn link_with_nested_parens_in_url() {
        let input = "go [here](https://en.wikipedia.org/wiki/Foo_(bar)) end";
        let chunks = tokenize(input);
        assert_eq!(
            chunks,
            vec![
                "go",
                "[here](https://en.wikipedia.org/wiki/Foo_(bar))",
                "end"
            ]
        );
    }

    #[test]
    fn reflow_preserves_atoms_wider_than_width() {
        let input = "a [very long link text](https://example.com/with/path) b";
        let out = reflow(input, 20);
        assert_eq!(
            out,
            "a\n[very long link text](https://example.com/with/path)\nb"
        );
    }

    #[test]
    fn idempotent_on_already_wrapped() {
        let input = "one two three four five six seven eight nine ten";
        let once = reflow(input, 15);
        let twice = reflow(&once, 15);
        assert_eq!(once, twice);
    }

    #[test]
    fn handles_utf8_multibyte() {
        let input = "café naïve résumé piñata";
        let out = reflow(input, 80);
        assert_eq!(out, "café naïve résumé piñata");
    }

    #[test]
    fn newlines_treated_as_whitespace() {
        let input = "one\ntwo\nthree";
        assert_eq!(reflow(input, 80), "one two three");
    }
}
