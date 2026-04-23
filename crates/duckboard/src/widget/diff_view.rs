//! Inline diff view with syntax highlighting and text selection.
//!
//! Builds an EditorState from diff data so it can be rendered via the TextEdit
//! widget, inheriting selection, copy, and horizontal scroll for free.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::highlight::{HighlightSpan, SyntaxHighlighter};
use crate::theme;
use crate::vcs::{self, DiffData, DiffLine, FileStatus, LineKind};

use super::text_edit::{EditorState, LineBgKind};

/// Materialised diff tab content: the editor plus the path/status fields a
/// `TabView::Diff` carries. Returns `None` when the file no longer differs
/// from HEAD (e.g. the change was just committed externally). `diff_data`
/// is retained so an async syntect highlight can rebuild editor spans via
/// [`compute_diff_highlight`] + [`build_diff_spans`].
pub struct DiffTabContent {
    pub editor: EditorState,
    pub path: PathBuf,
    pub status: FileStatus,
    pub diff_data: Arc<DiffData>,
}

/// Build a diff tab for `rel_path` with no syntax highlighting — only the
/// fallback muted/primary/secondary colors already baked into
/// [`build_diff_spans`]. The caller is expected to kick off an async job
/// that produces a [`DiffHighlight`] and hands it back to replace
/// `editor.highlight_spans` via [`build_diff_spans`].
pub fn build_diff_tab(repo_root: &Path, rel_path: &Path) -> Option<DiffTabContent> {
    let diff = vcs::file_diff(repo_root, rel_path)?;
    let path = diff.path.clone();
    let status = diff.status;
    let editor = build_editor(&diff, None);
    Some(DiffTabContent {
        editor,
        path,
        status,
        diff_data: Arc::new(diff),
    })
}

/// Run syntect over both sides of a diff. Pure function so it can be
/// called from a blocking task without touching any GUI state.
pub fn compute_diff_highlight(
    diff: &DiffData,
    ext: &str,
    highlighter: &SyntaxHighlighter,
) -> DiffHighlight {
    let syntax = highlighter.find_syntax(ext);
    let old_lines: Vec<String> = diff.old_content.lines().map(String::from).collect();
    let new_lines: Vec<String> = diff.new_content.lines().map(String::from).collect();
    DiffHighlight {
        old_spans: highlighter.highlight_lines(&old_lines, syntax),
        new_spans: highlighter.highlight_lines(&new_lines, syntax),
    }
}

/// Pre-computed syntax highlight data for a diff.
#[derive(Debug, Clone)]
pub struct DiffHighlight {
    /// Highlighted spans per line of the old file (index 0 = line 1).
    pub old_spans: Vec<Vec<HighlightSpan>>,
    /// Highlighted spans per line of the new file (index 0 = line 1).
    pub new_spans: Vec<Vec<HighlightSpan>>,
}

/// Build a read-only EditorState from diff data. When `highlight` is
/// `None`, fallback colors (muted/primary/secondary) are used instead of
/// syntect output so the tab can open instantly while highlighting runs
/// on a background task.
pub fn build_editor(diff: &DiffData, highlight: Option<&DiffHighlight>) -> EditorState {
    let mut lines = Vec::new();
    let mut backgrounds: Vec<Option<LineBgKind>> = Vec::new();

    for hunk in &diff.hunks {
        lines.push(hunk.header.trim_end().to_string());
        backgrounds.push(Some(LineBgKind::Hunk));
        for dl in &hunk.lines {
            let bg = match dl.kind {
                LineKind::Added => Some(LineBgKind::Added),
                LineKind::Removed => Some(LineBgKind::Removed),
                LineKind::Context => None,
            };
            let line_text = dl.text.trim_end_matches('\n');
            let lineno_str = format_lineno(dl);
            let (prefix, _, _) = line_prefix(dl.kind);
            lines.push(format!("{lineno_str}{prefix}{line_text}"));
            backgrounds.push(bg);
        }
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    let spans = build_diff_spans(diff, highlight);

    let mut editor = EditorState::new("");
    editor.lines = Arc::new(lines);
    editor.highlight_spans = Some(spans);
    editor.line_backgrounds = backgrounds;
    editor
}

/// Build only the per-line `highlight_spans` vector for a diff, given the
/// same `DiffData` the editor was constructed from and an optional
/// [`DiffHighlight`] carrying the syntect-highlighted source spans. The
/// returned vec aligns 1:1 with `build_editor(diff, _).lines`, so callers
/// can drop it into `editor.highlight_spans` directly. Separated from
/// [`build_editor`] so async handlers can refresh colors (e.g. after a
/// theme toggle or when a background syntect job completes) without
/// rebuilding `lines`/`line_backgrounds`.
pub fn build_diff_spans(
    diff: &DiffData,
    highlight: Option<&DiffHighlight>,
) -> Vec<Vec<HighlightSpan>> {
    let mut spans_per_line: Vec<Vec<HighlightSpan>> = Vec::new();

    for hunk in &diff.hunks {
        spans_per_line.push(vec![HighlightSpan {
            text: hunk.header.trim_end().to_string(),
            color: theme::text_muted(),
        }]);

        for dl in &hunk.lines {
            let (prefix, prefix_color, _bg) = line_prefix(dl.kind);
            let line_text = dl.text.trim_end_matches('\n');
            let lineno_str = format_lineno(dl);

            let mut line_spans = vec![
                HighlightSpan {
                    text: lineno_str,
                    color: theme::text_muted(),
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
                    LineKind::Added | LineKind::Removed => theme::text_primary(),
                    LineKind::Context => theme::text_secondary(),
                };
                line_spans.push(HighlightSpan {
                    text: line_text.to_string(),
                    color: fallback_color,
                });
            }

            spans_per_line.push(line_spans);
        }
    }

    spans_per_line
}

fn line_prefix(kind: LineKind) -> (&'static str, iced::Color, Option<LineBgKind>) {
    match kind {
        LineKind::Added => ("+ ", theme::success(), Some(LineBgKind::Added)),
        LineKind::Removed => ("- ", theme::error(), Some(LineBgKind::Removed)),
        LineKind::Context => ("  ", theme::text_muted(), None),
    }
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
