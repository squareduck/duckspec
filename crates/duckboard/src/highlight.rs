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
        use syntect::easy::HighlightLines;

        let theme = self.active_theme();
        let mut h = HighlightLines::new(syntax, theme);
        lines
            .iter()
            .map(|line| {
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
            })
            .collect()
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
