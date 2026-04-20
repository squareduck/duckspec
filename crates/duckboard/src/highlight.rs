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

const CATPPUCCIN_MACCHIATO: &[u8] =
    include_bytes!("../assets/catppuccin-macchiato.tmTheme");
const CATPPUCCIN_LATTE: &[u8] =
    include_bytes!("../assets/catppuccin-latte.tmTheme");

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
                md_spans
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
            .map(|s| (s.color.r.to_bits(), s.color.g.to_bits(), s.color.b.to_bits()))
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
        let src = lines(concat!(
            "~~~nosuchlang\n",
            "raw body\n",
            "~~~\n",
            "prose\n",
        ));
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
    fn is_closing_fence_requires_matching_char_and_length() {
        assert!(is_closing_fence("```", '`', 3));
        assert!(is_closing_fence("````", '`', 3));
        assert!(!is_closing_fence("``", '`', 3));
        assert!(!is_closing_fence("```rust", '`', 3)); // info string disqualifies
        assert!(!is_closing_fence("~~~", '`', 3));
    }
}
