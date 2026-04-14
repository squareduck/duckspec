//! Inline diff view with syntax highlighting and text selection.
//!
//! Builds an EditorState from diff data so it can be rendered via the TextEdit
//! widget, inheriting selection, copy, and horizontal scroll for free.

use iced::Color;

use crate::highlight::HighlightSpan;
use crate::theme;
use crate::vcs::{DiffData, DiffLine, LineKind};

use super::text_edit::EditorState;

/// Pre-computed syntax highlight data for a diff.
#[derive(Debug, Clone)]
pub struct DiffHighlight {
    /// Highlighted spans per line of the old file (index 0 = line 1).
    pub old_spans: Vec<Vec<HighlightSpan>>,
    /// Highlighted spans per line of the new file (index 0 = line 1).
    pub new_spans: Vec<Vec<HighlightSpan>>,
}

/// Build a read-only EditorState from diff data with syntax highlighting.
pub fn build_editor(diff: &DiffData, highlight: Option<&DiffHighlight>) -> EditorState {
    let mut lines = Vec::new();
    let mut backgrounds: Vec<Option<Color>> = Vec::new();
    let mut spans_per_line: Vec<Vec<HighlightSpan>> = Vec::new();

    for hunk in &diff.hunks {
        // Hunk header line.
        lines.push(hunk.header.trim_end().to_string());
        backgrounds.push(Some(theme::DIFF_HUNK_BG));
        spans_per_line.push(vec![HighlightSpan {
            text: hunk.header.trim_end().to_string(),
            color: theme::TEXT_MUTED,
        }]);

        for dl in &hunk.lines {
            let (prefix, prefix_color, bg) = match dl.kind {
                LineKind::Added => ("+ ", theme::SUCCESS, Some(theme::DIFF_ADDED_BG)),
                LineKind::Removed => ("- ", theme::ERROR, Some(theme::DIFF_REMOVED_BG)),
                LineKind::Context => ("  ", theme::TEXT_MUTED, None),
            };

            let line_text = dl.text.trim_end_matches('\n');
            let lineno_str = format_lineno(dl);
            let full_line = format!("{lineno_str}{prefix}{line_text}");
            lines.push(full_line);
            backgrounds.push(bg);

            // Build highlight spans: lineno + sign prefix + content spans.
            let mut line_spans = vec![
                HighlightSpan {
                    text: lineno_str.clone(),
                    color: theme::TEXT_MUTED,
                },
                HighlightSpan {
                    text: prefix.to_string(),
                    color: prefix_color,
                },
            ];

            let content_spans = highlight.and_then(|h| lookup_spans(dl, h));
            if let Some(src_spans) = content_spans {
                for s in src_spans {
                    line_spans.push(HighlightSpan {
                        text: s.text.trim_end_matches('\n').to_string(),
                        color: s.color,
                    });
                }
            } else {
                let fallback_color = match dl.kind {
                    LineKind::Added | LineKind::Removed => theme::TEXT_PRIMARY,
                    LineKind::Context => theme::TEXT_SECONDARY,
                };
                line_spans.push(HighlightSpan {
                    text: line_text.to_string(),
                    color: fallback_color,
                });
            }

            spans_per_line.push(line_spans);
        }
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    let mut editor = EditorState::new("");
    editor.lines = lines;
    editor.highlight_spans = Some(spans_per_line);
    editor.line_backgrounds = backgrounds;
    editor
}

fn format_lineno(dl: &DiffLine) -> String {
    let old = match dl.old_lineno {
        Some(n) => format!("{n:>4}"),
        None => "    ".to_string(),
    };
    let new = match dl.new_lineno {
        Some(n) => format!("{n:>4}"),
        None => "    ".to_string(),
    };
    format!("{old} {new} ")
}

fn lookup_spans<'a>(dl: &DiffLine, h: &'a DiffHighlight) -> Option<&'a Vec<HighlightSpan>> {
    match dl.kind {
        LineKind::Removed | LineKind::Context => {
            let idx = dl.old_lineno? as usize;
            h.old_spans.get(idx.checked_sub(1)?)
        }
        LineKind::Added => {
            let idx = dl.new_lineno? as usize;
            h.new_spans.get(idx.checked_sub(1)?)
        }
    }
}
