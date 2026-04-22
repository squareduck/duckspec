//! Syntax highlighting powered by syntect.
//!
//! Provides a shared highlighter resource and per-line colored spans that the
//! text editor widget renders via multiple `fill_text` calls.
//!
//! Supports both dark (Catppuccin Macchiato) and light (Catppuccin Latte)
//! themes, selected dynamically via [`crate::theme::mode()`].

use iced::Color;
use syntect::highlighting::{Style, Theme, ThemeSet};
use syntect::parsing::{SyntaxReference, SyntaxSet};

use crate::theme;

const CATPPUCCIN_MACCHIATO: &[u8] = include_bytes!("../assets/catppuccin-macchiato.tmTheme");
const CATPPUCCIN_LATTE: &[u8] = include_bytes!("../assets/catppuccin-latte.tmTheme");

/// App-level highlighter holding the (expensive) syntax set and both themes.
pub struct SyntaxHighlighter {
    syntax_set: SyntaxSet,
    dark_theme: Theme,
    light_theme: Theme,
}

/// A single colored text span within a line.
#[derive(Debug, Clone)]
pub struct HighlightSpan {
    pub text: String,
    pub color: Color,
}

impl Default for SyntaxHighlighter {
    fn default() -> Self {
        Self::new()
    }
}

fn load_theme(bytes: &[u8]) -> Theme {
    ThemeSet::load_from_reader(&mut std::io::Cursor::new(bytes))
        .expect("embedded tmTheme should be valid")
}

impl SyntaxHighlighter {
    pub fn new() -> Self {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        Self {
            syntax_set,
            dark_theme: load_theme(CATPPUCCIN_MACCHIATO),
            light_theme: load_theme(CATPPUCCIN_LATTE),
        }
    }

    fn active_theme(&self) -> &Theme {
        match theme::mode() {
            theme::ColorMode::Dark => &self.dark_theme,
            theme::ColorMode::Light => &self.light_theme,
        }
    }

    pub fn find_syntax(&self, file_extension: &str) -> &SyntaxReference {
        self.syntax_set
            .find_syntax_by_extension(file_extension)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
    }

    /// Highlight all lines, returning a vec of colored spans per line.
    pub fn highlight_lines(
        &self,
        lines: &[String],
        syntax: &SyntaxReference,
    ) -> Vec<Vec<HighlightSpan>> {
        if syntax.name == "Markdown" {
            self.highlight_markdown(lines, syntax)
        } else {
            self.highlight_plain(lines, syntax)
        }
    }

    fn highlight_plain(
        &self,
        lines: &[String],
        syntax: &SyntaxReference,
    ) -> Vec<Vec<HighlightSpan>> {
        use syntect::easy::HighlightLines;

        let theme = self.active_theme();
        let mut h = HighlightLines::new(syntax, theme);
        lines
            .iter()
            .map(|line| self.line_spans(line, &mut h))
            .collect()
    }

    /// Markdown-aware highlighting: detect fenced code blocks and apply the
    /// embedded language's syntax to their bodies, while the fence lines and
    /// prose stay on the markdown grammar.
    fn highlight_markdown(
        &self,
        lines: &[String],
        md_syntax: &SyntaxReference,
    ) -> Vec<Vec<HighlightSpan>> {
        use syntect::easy::HighlightLines;

        let theme = self.active_theme();
        let inline_code_color = inline_code_color(theme);
        let mut md_h = HighlightLines::new(md_syntax, theme);
        let mut code: Option<CodeBlock> = None;
        let mut out = Vec::with_capacity(lines.len());

        for line in lines {
            let closing = code
                .as_ref()
                .is_some_and(|c| is_closing_fence(line, c.fence_char, c.fence_len));

            // Always advance the markdown parser so its state stays coherent.
            let md_spans = self.line_spans(line, &mut md_h);

            let spans = if let Some(c) = code.as_mut()
                && !closing
                && let Some(h) = c.hl.as_mut()
            {
                self.line_spans(line, h)
            } else {
                apply_inline_code(md_spans, line, inline_code_color)
            };

            out.push(spans);

            if closing {
                code = None;
            } else if code.is_none()
                && let Some((fc, fl, info)) = parse_opening_fence(line)
            {
                let lang = info
                    .split(|c: char| c.is_whitespace() || c == ',' || c == '{')
                    .next()
                    .unwrap_or("")
                    .to_ascii_lowercase();
                let hl = if lang.is_empty() {
                    None
                } else {
                    self.syntax_set
                        .find_syntax_by_token(&lang)
                        .map(|s| HighlightLines::new(s, theme))
                };
                code = Some(CodeBlock {
                    fence_char: fc,
                    fence_len: fl,
                    hl,
                });
            }
        }

        out
    }

    fn line_spans(
        &self,
        line: &str,
        h: &mut syntect::easy::HighlightLines<'_>,
    ) -> Vec<HighlightSpan> {
        let with_nl = format!("{line}\n");
        let ranges = h
            .highlight_line(&with_nl, &self.syntax_set)
            .unwrap_or_default();
        ranges
            .into_iter()
            .map(|(style, text)| HighlightSpan {
                text: text.trim_end_matches('\n').to_string(),
                color: syntect_to_iced(style),
            })
            .filter(|s| !s.text.is_empty())
            .collect()
    }
}

struct CodeBlock<'a> {
    fence_char: char,
    fence_len: usize,
    hl: Option<syntect::easy::HighlightLines<'a>>,
}

fn parse_opening_fence(line: &str) -> Option<(char, usize, &str)> {
    let trimmed = line.trim_start();
    let indent = line.len() - trimmed.len();
    if indent > 3 {
        return None;
    }
    let fc = trimmed.chars().next()?;
    if fc != '`' && fc != '~' {
        return None;
    }
    let fence_len = trimmed.chars().take_while(|&c| c == fc).count();
    if fence_len < 3 {
        return None;
    }
    let info = trimmed[fence_len..].trim();
    // For backtick fences, the info string must not contain a backtick.
    if fc == '`' && info.contains('`') {
        return None;
    }
    Some((fc, fence_len, info))
}

fn is_closing_fence(line: &str, fence_char: char, fence_len: usize) -> bool {
    let trimmed = line.trim_start();
    let indent = line.len() - trimmed.len();
    if indent > 3 {
        return false;
    }
    let run = trimmed.chars().take_while(|&c| c == fence_char).count();
    if run < fence_len {
        return false;
    }
    // Closing fence must have no info string (only whitespace after the run).
    trimmed[run..].trim().is_empty()
}

/// Look up the theme color for `markup.inline.raw.string.markdown`.
fn inline_code_color(theme: &Theme) -> Color {
    use std::str::FromStr;
    use syntect::highlighting::Highlighter;
    use syntect::parsing::ScopeStack;

    let highlighter = Highlighter::new(theme);
    let stack = ScopeStack::from_str("text.html.markdown markup.inline.raw.string.markdown")
        .expect("valid scope path");
    let style = highlighter.style_for_stack(stack.as_slice());
    syntect_to_iced(style)
}

/// Find byte ranges of CommonMark inline code spans in `line`. Each range
/// covers the opening backticks, content, and closing backticks. Backtick
/// runs only match a closing run of the same length. Backslash-escaped
/// backticks outside of a code span do not start a run.
fn find_inline_code_runs(line: &str) -> Vec<(usize, usize)> {
    let bytes = line.as_bytes();
    let mut runs = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() && bytes[i + 1] == b'`' {
            i += 2;
            continue;
        }
        if bytes[i] != b'`' {
            i += 1;
            continue;
        }
        let start = i;
        let open_len = bytes[i..].iter().take_while(|&&b| b == b'`').count();
        let body_start = start + open_len;
        let mut j = body_start;
        let mut closed = false;
        while j < bytes.len() {
            if bytes[j] != b'`' {
                j += 1;
                continue;
            }
            let close_len = bytes[j..].iter().take_while(|&&b| b == b'`').count();
            if close_len == open_len {
                runs.push((start, j + close_len));
                i = j + close_len;
                closed = true;
                break;
            }
            j += close_len;
        }
        if !closed {
            i = body_start;
        }
    }
    runs
}

/// Recolor the parts of `spans` that fall inside `line`'s inline code runs.
/// The concatenation of `spans` text must equal `line`.
fn apply_inline_code(
    spans: Vec<HighlightSpan>,
    line: &str,
    color: Color,
) -> Vec<HighlightSpan> {
    let runs = find_inline_code_runs(line);
    if runs.is_empty() {
        return spans;
    }
    let mut out = Vec::with_capacity(spans.len());
    let mut byte_pos = 0usize;
    for span in spans {
        let span_start = byte_pos;
        let span_end = byte_pos + span.text.len();
        let mut cursor = span_start;
        while cursor < span_end {
            let in_run = runs.iter().find(|(s, e)| cursor >= *s && cursor < *e);
            let (next_boundary, use_color) = if let Some((_, end)) = in_run {
                ((*end).min(span_end), color)
            } else {
                let next_run_start = runs
                    .iter()
                    .map(|(s, _)| *s)
                    .find(|s| *s > cursor)
                    .unwrap_or(span_end);
                (next_run_start.min(span_end), span.color)
            };
            let text = &line[cursor..next_boundary];
            if !text.is_empty() {
                push_span(&mut out, text, use_color);
            }
            cursor = next_boundary;
        }
        byte_pos = span_end;
    }
    out
}

fn push_span(out: &mut Vec<HighlightSpan>, text: &str, color: Color) {
    if let Some(last) = out.last_mut()
        && last.color == color
    {
        last.text.push_str(text);
    } else {
        out.push(HighlightSpan {
            text: text.to_string(),
            color,
        });
    }
}

fn syntect_to_iced(style: Style) -> Color {
    Color {
        r: style.foreground.r as f32 / 255.0,
        g: style.foreground.g as f32 / 255.0,
        b: style.foreground.b as f32 / 255.0,
        a: style.foreground.a as f32 / 255.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(s: &str) -> Vec<String> {
        s.lines().map(String::from).collect()
    }

    fn concat(spans: &[HighlightSpan]) -> String {
        spans.iter().map(|s| s.text.as_str()).collect()
    }

    /// Colors differ when two lines are highlighted by different grammars.
    /// We check that code-block-interior lines are styled differently from
    /// prose in the same file: if the markdown grammar had been applied to
    /// them, every non-leading-whitespace span would share a single plain
    /// "raw code" color.
    #[test]
    fn fenced_code_block_gets_language_colors() {
        let hl = SyntaxHighlighter::new();
        let md = hl.find_syntax("md");
        let src = lines(concat!(
            "# Title\n",
            "\n",
            "```rust\n",
            "fn main() { let x = 42; }\n",
            "```\n",
        ));
        let out = hl.highlight_lines(&src, md);
        assert_eq!(out.len(), src.len());

        // Line 3 is the Rust code. With Rust highlighting applied, we
        // expect multiple distinct colors (keyword, identifier, number).
        let code_spans = &out[3];
        assert_eq!(concat(code_spans), "fn main() { let x = 42; }");
        let distinct_colors: std::collections::HashSet<_> = code_spans
            .iter()
            .map(|s| {
                (
                    s.color.r.to_bits(),
                    s.color.g.to_bits(),
                    s.color.b.to_bits(),
                )
            })
            .collect();
        assert!(
            distinct_colors.len() >= 3,
            "expected Rust highlighting to produce >=3 distinct colors, got {}: {:?}",
            distinct_colors.len(),
            code_spans
        );
    }

    #[test]
    fn tilde_fence_and_unknown_language() {
        let hl = SyntaxHighlighter::new();
        let md = hl.find_syntax("md");
        let src = lines(concat!("~~~nosuchlang\n", "raw body\n", "~~~\n", "prose\n",));
        let out = hl.highlight_lines(&src, md);
        assert_eq!(out.len(), src.len());
        // Unknown language: body falls back to markdown styling — we just
        // want to verify it doesn't panic and produces spans.
        assert_eq!(concat(&out[1]), "raw body");
        assert_eq!(concat(&out[3]), "prose");
    }

    #[test]
    fn closing_fence_ends_code_block() {
        let hl = SyntaxHighlighter::new();
        let md = hl.find_syntax("md");
        let src = lines(concat!(
            "```python\n",
            "x = 1\n",
            "```\n",
            "```python\n",
            "y = 2\n",
            "```\n",
        ));
        let out = hl.highlight_lines(&src, md);
        // Both code-body lines should be highlighted — if the first
        // closing fence wasn't honored, the second fence line would be
        // treated as code.
        assert_eq!(concat(&out[1]), "x = 1");
        assert_eq!(concat(&out[4]), "y = 2");
    }

    #[test]
    fn parse_opening_fence_accepts_common_forms() {
        assert_eq!(parse_opening_fence("```rust"), Some(('`', 3, "rust")));
        assert_eq!(parse_opening_fence("   ```rust"), Some(('`', 3, "rust")));
        assert_eq!(parse_opening_fence("~~~~py"), Some(('~', 4, "py")));
        assert_eq!(parse_opening_fence("``not a fence"), None);
        assert_eq!(parse_opening_fence("    ```rust"), None); // 4-space indent
        assert_eq!(parse_opening_fence("```rs`bad"), None); // backtick in info
    }

    #[test]
    fn inline_code_spans_get_distinct_color() {
        let hl = SyntaxHighlighter::new();
        let md = hl.find_syntax("md");
        let src = lines("Some prose with `inline code` in it.\n");
        let out = hl.highlight_lines(&src, md);
        assert_eq!(concat(&out[0]), "Some prose with `inline code` in it.");

        let prose_color = out[0]
            .iter()
            .find(|s| s.text.contains("Some prose"))
            .map(|s| s.color)
            .unwrap();
        let code_span = out[0]
            .iter()
            .find(|s| s.text == "`inline code`")
            .expect("inline code region should be a single span");
        assert_ne!(
            (code_span.color.r, code_span.color.g, code_span.color.b),
            (prose_color.r, prose_color.g, prose_color.b),
            "inline code should be colored distinctly from prose"
        );
    }

    #[test]
    fn inline_code_with_double_backticks_and_embedded_single() {
        let hl = SyntaxHighlighter::new();
        let md = hl.find_syntax("md");
        let src = lines("text ``a `b` c`` end\n");
        let out = hl.highlight_lines(&src, md);
        assert_eq!(concat(&out[0]), "text ``a `b` c`` end");
        let code = out[0]
            .iter()
            .find(|s| s.text == "``a `b` c``")
            .expect("double-backtick span should not be split by inner single backticks");
        let prose = out[0].iter().find(|s| s.text.starts_with("text")).unwrap();
        assert_ne!(code.color, prose.color);
    }

    #[test]
    fn inline_code_skipped_inside_fenced_block() {
        let hl = SyntaxHighlighter::new();
        let md = hl.find_syntax("md");
        let src = lines(concat!("```\n", "let s = `not inline`;\n", "```\n",));
        let out = hl.highlight_lines(&src, md);
        // Code-block body should be untouched by inline-code post-processing.
        assert_eq!(concat(&out[1]), "let s = `not inline`;");
    }

    #[test]
    fn unmatched_backtick_is_left_alone() {
        let hl = SyntaxHighlighter::new();
        let md = hl.find_syntax("md");
        let src = lines("a ` lonely backtick here\n");
        let out = hl.highlight_lines(&src, md);
        assert_eq!(concat(&out[0]), "a ` lonely backtick here");
    }

    #[test]
    fn find_inline_code_runs_basics() {
        assert_eq!(find_inline_code_runs("a `b` c"), vec![(2, 5)]);
        assert_eq!(find_inline_code_runs("``a `x` b``"), vec![(0, 11)]);
        assert_eq!(find_inline_code_runs("no code here"), vec![]);
        assert_eq!(find_inline_code_runs("a ` lonely"), vec![]);
        assert_eq!(find_inline_code_runs("a \\`escaped\\` b"), vec![]);
        assert_eq!(
            find_inline_code_runs("`one` and `two`"),
            vec![(0, 5), (10, 15)]
        );
    }

    #[test]
    fn is_closing_fence_requires_matching_char_and_length() {
        assert!(is_closing_fence("```", '`', 3));
        assert!(is_closing_fence("````", '`', 3));
        assert!(!is_closing_fence("``", '`', 3));
        assert!(!is_closing_fence("```rust", '`', 3)); // info string disqualifies
        assert!(!is_closing_fence("~~~", '`', 3));
    }
}
