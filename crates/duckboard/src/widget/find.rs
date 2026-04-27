//! Local find: regex search within a single editor file or chat session.
//!
//! Cmd-F opens a modal that previews matches as the user types. Enter commits
//! the query: the modal closes, a per-target [`FindState`] is stored on the
//! app, and a status toolbar appears under the file/session header. Toolbar
//! shows the live query, candidate count, prev/next buttons, and a cancel.
//! Ctrl-N/P navigate. Esc / click-away on the modal also clears any active
//! find for the same target so cmd-f always enters a fresh "create" mode.

use std::path::PathBuf;

use iced::Task;
use iced::advanced::widget::{Id, Operation, operation};
use iced::widget::text::Span;
use iced::widget::{Space, button, column, container, mouse_area, rich_text, row, scrollable, span, text, text_input};
use iced::{Center, Color, Element, Font, Length, Rectangle, Vector};
use regex::Regex;

use crate::theme;
use crate::widget::text_edit::EditorState;

const MAX_PREVIEW_LEN: usize = 300;
pub const FIND_INPUT_ID: &str = "find-modal-input";

// ── Target ─────────────────────────────────────────────────────────────────

/// What the find is scoped to. Editor finds key by tab id (file path or
/// pseudo-id like `idea:…`); chat finds key by `(instance_id, session_id)`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FindTarget {
    Editor(String),
    ChatSession(u64, String),
}

impl FindTarget {
    pub fn editor(tab_id: impl Into<String>) -> Self {
        FindTarget::Editor(tab_id.into())
    }

    pub fn chat(instance_id: u64, session_id: impl Into<String>) -> Self {
        FindTarget::ChatSession(instance_id, session_id.into())
    }
}

// ── Match record ───────────────────────────────────────────────────────────

/// One match within a find target. `byte_range` is into the line text, not
/// the whole document — keeps the highlight plumbing identical for editor
/// and chat (which stores per-block line buffers).
#[derive(Debug, Clone)]
pub struct FindMatch {
    /// 0-based line index within the target. For chat finds this is global
    /// across all blocks (the editor stores per-block flat lines).
    pub line: usize,
    /// Byte range within `line_text` that the match spans.
    pub byte_start: usize,
    pub byte_end: usize,
    /// Truncated/centered preview of the match line.
    pub line_text: String,
    /// For chat finds: index into `chat_blocks`. `None` for editor finds.
    pub block_idx: Option<usize>,
    /// For chat finds: human-readable role label of the block (User /
    /// Assistant / Tool / etc.). `None` for editor finds.
    pub block_role: Option<&'static str>,
}

// ── Per-target find state ──────────────────────────────────────────────────

/// Live find state kept once a query is committed (Enter from the modal).
/// Cleared by Esc on the modal, the toolbar's cancel button, or a fresh
/// cmd-f opening the modal again.
#[derive(Debug, Clone)]
pub struct FindState {
    pub query: String,
    pub matches: Vec<FindMatch>,
    pub current: usize,
}

impl FindState {
    pub fn current_match(&self) -> Option<&FindMatch> {
        self.matches.get(self.current)
    }

    pub fn select_next(&mut self) {
        if self.matches.is_empty() {
            return;
        }
        self.current = (self.current + 1) % self.matches.len();
    }

    pub fn select_prev(&mut self) {
        if self.matches.is_empty() {
            return;
        }
        if self.current == 0 {
            self.current = self.matches.len() - 1;
        } else {
            self.current -= 1;
        }
    }
}

// ── Modal state ────────────────────────────────────────────────────────────

/// Snapshot of the target the modal is operating on, captured on open. The
/// modal can compute live previews from this without holding borrows on the
/// app state across UI rebuilds. We re-derive the actual matches on commit
/// from the live editor/chat state, so concurrent edits during preview don't
/// produce stale match positions.
#[derive(Debug, Clone)]
pub struct ModalSnapshot {
    pub target: FindTarget,
    /// Display label for the target (file path or session name); shown as a
    /// hint above the input.
    pub target_label: String,
    /// Lines snapshotted from the target. For editor: editor.lines. For
    /// chat: flattened block lines with role labels resolved.
    pub lines: Vec<String>,
    /// For chat: per-line (block_idx, role) so previews can show the role.
    /// Empty for editor finds.
    pub line_meta: Vec<(usize, &'static str)>,
}

#[derive(Debug, Default)]
pub struct FindModalState {
    pub visible: bool,
    pub query: String,
    pub snapshot: Option<ModalSnapshot>,
    /// Live preview matches computed from the snapshot; rebuilt on every
    /// query change. Up to PREVIEW_LIMIT entries.
    pub preview: Vec<FindMatch>,
    pub selected: usize,
    /// Last regex compile error, shown inline in the modal so the user can
    /// see why their query has no results. None when query is empty or
    /// compiles cleanly.
    pub error: Option<String>,
}

const PREVIEW_LIMIT: usize = 200;

impl FindModalState {
    pub fn open(&mut self, snapshot: ModalSnapshot) {
        self.visible = true;
        self.query.clear();
        self.preview.clear();
        self.selected = 0;
        self.error = None;
        self.snapshot = Some(snapshot);
    }

    pub fn close(&mut self) {
        self.visible = false;
        self.query.clear();
        self.preview.clear();
        self.selected = 0;
        self.error = None;
        self.snapshot = None;
    }

    pub fn target(&self) -> Option<&FindTarget> {
        self.snapshot.as_ref().map(|s| &s.target)
    }

    pub fn set_query(&mut self, query: String) {
        self.query = query;
        self.recompute_preview();
    }

    pub fn select_next(&mut self) {
        if self.preview.is_empty() {
            return;
        }
        self.selected = (self.selected + 1).min(self.preview.len() - 1);
    }

    pub fn select_prev(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    fn recompute_preview(&mut self) {
        self.preview.clear();
        self.selected = 0;
        self.error = None;
        if self.query.is_empty() {
            return;
        }
        let Some(snap) = self.snapshot.as_ref() else {
            return;
        };
        let regex = match build_regex(&self.query) {
            Ok(r) => r,
            Err(e) => {
                self.error = Some(e);
                return;
            }
        };
        self.preview = compute_matches_in_lines(&snap.lines, &snap.line_meta, &regex, PREVIEW_LIMIT);
    }
}

// ── Search engine ──────────────────────────────────────────────────────────

/// Compile the user's query as a smart-case regex. Smart-case mirrors what
/// cmd-shift-f does (`grep_regex::case_smart`): lowercase queries are
/// case-insensitive, mixed-case queries are case-sensitive.
pub fn build_regex(query: &str) -> Result<Regex, String> {
    let case_sensitive = query.chars().any(|c| c.is_uppercase());
    let pattern = if case_sensitive {
        query.to_string()
    } else {
        format!("(?i){query}")
    };
    Regex::new(&pattern).map_err(|e| e.to_string())
}

/// Find all matches in a flat line buffer. `line_meta`, when non-empty,
/// supplies (block_idx, role) tags for chat finds. Capped at `limit`.
pub fn compute_matches_in_lines(
    lines: &[String],
    line_meta: &[(usize, &'static str)],
    regex: &Regex,
    limit: usize,
) -> Vec<FindMatch> {
    let mut out = Vec::new();
    for (line_idx, line) in lines.iter().enumerate() {
        if out.len() >= limit {
            break;
        }
        for m in regex.find_iter(line) {
            if out.len() >= limit {
                break;
            }
            let (line_text, (byte_start, byte_end)) =
                truncate_preview(line.clone(), m.start(), m.end());
            let (block_idx, role) = match line_meta.get(line_idx) {
                Some((b, r)) => (Some(*b), Some(*r)),
                None => (None, None),
            };
            out.push(FindMatch {
                line: line_idx,
                byte_start,
                byte_end,
                line_text,
                block_idx,
                block_role: role,
            });
        }
    }
    out
}

/// Build a snapshot from a single editor (file content tab).
pub fn snapshot_editor(target: FindTarget, label: String, editor: &EditorState) -> ModalSnapshot {
    let lines: Vec<String> = editor.lines.iter().cloned().collect();
    ModalSnapshot {
        target,
        target_label: label,
        lines,
        line_meta: Vec::new(),
    }
}

/// Build a snapshot from a chat session's per-block editors. The flattened
/// `lines` mirrors what the user sees: each block contributes its body
/// lines (header lines are skipped — they're decoration, not content the
/// user wrote). The per-line `line_meta` carries block index + role so
/// preview rows can show "[role #N]: …".
///
/// Tool-use / tool-result blocks are excluded — finds are scoped to the
/// human-readable conversation, not transient tool I/O.
pub fn snapshot_chat(
    target: FindTarget,
    label: String,
    block_editors: &[EditorState],
    block_roles: &[&'static str],
    block_searchable: &[bool],
) -> ModalSnapshot {
    let mut lines = Vec::new();
    let mut meta = Vec::new();
    for (block_idx, ed) in block_editors.iter().enumerate() {
        if !block_searchable.get(block_idx).copied().unwrap_or(true) {
            continue;
        }
        let role = block_roles.get(block_idx).copied().unwrap_or("");
        // Skip header line (line 0) — `EditorState::from_blocks` puts the
        // block label there, which is decoration. Body lines start at idx 1.
        for (line_idx, line) in ed.lines.iter().enumerate() {
            if line_idx == 0 && !ed.blocks.is_empty() {
                continue;
            }
            lines.push(line.clone());
            meta.push((block_idx, role));
        }
    }
    ModalSnapshot {
        target,
        target_label: label,
        lines,
        line_meta: meta,
    }
}

/// Find matches in a single editor (commit path).
pub fn matches_for_editor(query: &str, editor: &EditorState) -> Result<Vec<FindMatch>, String> {
    let regex = build_regex(query)?;
    let lines: Vec<String> = editor.lines.iter().cloned().collect();
    Ok(compute_matches_in_lines(&lines, &[], &regex, usize::MAX))
}

/// Find matches in a chat session (commit path). Maps each per-block editor
/// line through `block_idx` so the toolbar's prev/next can scroll to the
/// matching block + line. Skips blocks whose `block_searchable` is false
/// (tool calls/results).
pub fn matches_for_chat(
    query: &str,
    block_editors: &[EditorState],
    block_roles: &[&'static str],
    block_searchable: &[bool],
) -> Result<Vec<FindMatch>, String> {
    let regex = build_regex(query)?;
    let mut out = Vec::new();
    for (block_idx, ed) in block_editors.iter().enumerate() {
        if !block_searchable.get(block_idx).copied().unwrap_or(true) {
            continue;
        }
        let role = block_roles.get(block_idx).copied().unwrap_or("");
        for (line_idx, line) in ed.lines.iter().enumerate() {
            if line_idx == 0 && !ed.blocks.is_empty() {
                continue;
            }
            for m in regex.find_iter(line) {
                let (line_text, (byte_start, byte_end)) =
                    truncate_preview(line.clone(), m.start(), m.end());
                out.push(FindMatch {
                    line: line_idx,
                    byte_start,
                    byte_end,
                    line_text,
                    block_idx: Some(block_idx),
                    block_role: Some(role),
                });
            }
        }
    }
    Ok(out)
}

// ── Project-root file ergonomics ───────────────────────────────────────────

/// Build a display label for an editor target from its tab id and the
/// project root. Mirrors what cmd-shift-f uses.
pub fn editor_label_for(tab_id: &str, project_root: Option<&std::path::Path>) -> String {
    if let Some(rel) = tab_id.strip_prefix("file:") {
        if let Some(_root) = project_root {
            return rel.to_string();
        }
        return rel.to_string();
    }
    tab_id.to_string()
}

/// Translate an absolute file path into a project-relative display label,
/// falling back to the absolute path if the file is outside the root.
#[allow(dead_code)]
pub fn rel_or_abs(path: &PathBuf, project_root: Option<&std::path::Path>) -> String {
    if let Some(root) = project_root {
        if let Ok(rel) = path.strip_prefix(root) {
            return rel.display().to_string();
        }
    }
    path.display().to_string()
}

// ── Preview truncation (mirrors text_search) ───────────────────────────────

fn truncate_preview(line: String, start: usize, end: usize) -> (String, (usize, usize)) {
    if line.len() <= MAX_PREVIEW_LEN {
        return (line, (start, end));
    }
    let margin = 40;
    let begin = start.saturating_sub(margin);
    let take = MAX_PREVIEW_LEN.min(line.len() - begin);
    let begin = snap_char_boundary(&line, begin);
    let end_cut = snap_char_boundary(&line, begin + take);
    let sliced = &line[begin..end_cut];
    let prefix = if begin > 0 { "… " } else { "" };
    let suffix = if end_cut < line.len() { " …" } else { "" };
    let shift = prefix.len();
    let new_line = format!("{prefix}{sliced}{suffix}");
    let new_start = (start.saturating_sub(begin)) + shift;
    let new_end = (end.saturating_sub(begin)) + shift;
    let new_end = new_end.min(new_line.len());
    let new_start = new_start.min(new_end);
    (new_line, (new_start, new_end))
}

fn snap_char_boundary(s: &str, mut i: usize) -> usize {
    while i < s.len() && !s.is_char_boundary(i) {
        i += 1;
    }
    i.min(s.len())
}

// ── Messages ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Msg {
    /// Live edit of the modal's query input.
    QueryChanged(String),
    /// Move preview selection in the modal.
    PreviewSelectNext,
    PreviewSelectPrev,
    /// Enter pressed in the modal — commit the current query, close the
    /// modal, and activate the toolbar / highlights.
    Commit,
    /// Esc / click-away — close the modal *and* clear any active find for
    /// the modal's target. The user's contract: cmd-f always enters fresh
    /// "create" mode.
    Cancel,
    /// Toolbar prev/next button, or ctrl-n/p in committed mode.
    Navigate(FindTarget, NavDir),
    /// Toolbar X button — clear the active find for this target.
    Deactivate(FindTarget),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavDir {
    Next,
    Prev,
}

// ── Modal view ─────────────────────────────────────────────────────────────

const MAX_VISIBLE_PREVIEW: usize = 50;

pub fn view_modal<'a>(state: &'a FindModalState) -> Element<'a, Msg> {
    let target_label = state
        .snapshot
        .as_ref()
        .map(|s| s.target_label.as_str())
        .unwrap_or("");
    let scope_kind_label = match state.snapshot.as_ref().map(|s| &s.target) {
        Some(FindTarget::Editor(_)) => "in file",
        Some(FindTarget::ChatSession(_, _)) => "in chat",
        None => "",
    };
    let scope_label = container(
        row![
            text(scope_kind_label)
                .size(theme::font_sm())
                .color(theme::text_muted()),
            text(target_label)
                .size(theme::font_sm())
                .font(theme::content_font())
                .color(theme::text_secondary()),
        ]
        .spacing(theme::SPACING_XS)
        .align_y(Center),
    )
    .padding([theme::SPACING_SM, theme::SPACING_MD]);

    let placeholder = match state.snapshot.as_ref().map(|s| &s.target) {
        Some(FindTarget::ChatSession(_, _)) => "Find in chat (regex)…",
        _ => "Find in file (regex)…",
    };
    let input = text_input(placeholder, &state.query)
        .on_input(Msg::QueryChanged)
        .on_submit(Msg::Commit)
        .size(theme::font_md())
        .font(theme::content_font())
        .padding([theme::SPACING_SM, theme::SPACING_MD])
        .width(Length::Fill)
        .style(finder_input_style)
        .id(FIND_INPUT_ID);

    let hdivider = || container(Space::new().height(1.0).width(Length::Fill)).style(divider_style);

    let mut list = column![].spacing(0.0);
    let visible_count = state.preview.len().min(MAX_VISIBLE_PREVIEW);
    for (idx, hit) in state.preview.iter().take(MAX_VISIBLE_PREVIEW).enumerate() {
        let is_selected = idx == state.selected;
        let row_style: fn(&iced::Theme) -> container::Style = if is_selected {
            selected_item_style
        } else {
            item_style
        };
        let base_color = if is_selected {
            theme::text_primary()
        } else {
            theme::text_secondary()
        };
        let path_label = match (hit.block_idx, hit.block_role) {
            (Some(bi), Some(role)) => format!("[{role} #{}]:{} ", bi + 1, hit.line + 1),
            _ => format!("L{}: ", hit.line + 1),
        };
        let mut spans: Vec<Span<'_, (), Font>> = Vec::new();
        spans.push(span(path_label).color(theme::text_muted()));
        spans.extend(highlight_match_spans(
            &hit.line_text,
            hit.byte_start,
            hit.byte_end,
            base_color,
            theme::accent(),
        ));
        list = list.push(
            container(
                rich_text(spans)
                    .size(theme::font_sm())
                    .font(theme::content_font())
                    .on_link_click(|_: ()| unreachable!("find modal has no links")),
            )
            .padding([theme::SPACING_XS, theme::SPACING_MD])
            .width(Length::Fill)
            .style(row_style),
        );
    }

    let more_hidden = state.preview.len().saturating_sub(visible_count);
    let left_text = if let Some(err) = state.error.as_ref() {
        format!("invalid regex: {err}")
    } else if state.query.is_empty() {
        "type to search".to_string()
    } else if state.preview.is_empty() {
        "no matches".to_string()
    } else if more_hidden > 0 {
        format!("{visible_count} matches (+{more_hidden} hidden)")
    } else {
        format!("{} matches", state.preview.len())
    };
    let left_color = if state.error.is_some() {
        theme::error()
    } else {
        theme::text_muted()
    };
    let left = text(left_text)
        .size(theme::font_sm())
        .font(theme::content_font())
        .color(left_color);
    let hints = text("⏎ activate   ⎋ cancel")
        .size(theme::font_sm())
        .font(theme::content_font())
        .color(theme::text_muted());
    let status_bar = container(
        row![left, Space::new().width(Length::Fill), hints].align_y(Center),
    )
    .padding([theme::SPACING_XS, theme::SPACING_MD])
    .width(Length::Fill);

    let panel = container(
        column![
            scope_label,
            hdivider(),
            input,
            hdivider(),
            scrollable(list)
                .direction(theme::thin_scrollbar_direction())
                .style(theme::thin_scrollbar)
                .height(Length::Shrink),
            status_bar,
        ]
        .spacing(0.0)
        .max_width(800.0),
    )
    .padding(1)
    .style(finder_panel_style)
    .max_width(800.0);

    // Click-away: a `mouse_area` covers the whole backdrop and emits Cancel.
    // Clicks on the panel are absorbed by the panel's child widgets
    // (text_input, list rows) so they never reach the backdrop's mouse_area.
    let backdrop = container(column![Space::new().height(80.0), panel].align_x(Center))
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Center)
        .style(overlay_backdrop_style);

    mouse_area(backdrop).on_press(Msg::Cancel).into()
}

// ── Toolbar view ───────────────────────────────────────────────────────────

/// Compact status strip rendered under the file path bar / chat session bar
/// when a find is active for that target. Shows the query, match count,
/// prev/next buttons, and a cancel.
pub fn view_toolbar<'a>(target: FindTarget, state: &'a FindState) -> Element<'a, Msg> {
    let total = state.matches.len();
    let position = if total == 0 {
        "0 / 0".to_string()
    } else {
        format!("{} / {}", state.current + 1, total)
    };

    let query_text = text(state.query.clone())
        .size(theme::font_sm())
        .font(theme::content_font())
        .color(theme::text_primary())
        .wrapping(iced::widget::text::Wrapping::None);

    let count_text = text(position)
        .size(theme::font_sm())
        .color(theme::text_muted());

    let prev_btn = button(text("‹").size(theme::font_sm()).color(theme::text_secondary()))
        .on_press(Msg::Navigate(target.clone(), NavDir::Prev))
        .padding([0.0, theme::SPACING_SM])
        .style(toolbar_btn_style);
    let next_btn = button(text("›").size(theme::font_sm()).color(theme::text_secondary()))
        .on_press(Msg::Navigate(target.clone(), NavDir::Next))
        .padding([0.0, theme::SPACING_SM])
        .style(toolbar_btn_style);
    let close_btn = button(text("×").size(theme::font_sm()).color(theme::text_muted()))
        .on_press(Msg::Deactivate(target))
        .padding([0.0, theme::SPACING_SM])
        .style(toolbar_btn_style);

    let label = text("find:")
        .size(theme::font_sm())
        .color(theme::text_muted());

    let bar = row![
        label,
        query_text,
        Space::new().width(Length::Fill),
        count_text,
        prev_btn,
        next_btn,
        close_btn,
    ]
    .spacing(theme::SPACING_XS)
    .align_y(Center);

    let bar = container(bar)
        .padding([2.0, theme::SPACING_SM])
        .width(Length::Fill)
        .style(toolbar_bar_style);

    column![
        bar,
        container(Space::new().width(Length::Fill).height(1.0)).style(divider_style),
    ]
    .into()
}

// ── Styles ─────────────────────────────────────────────────────────────────

fn overlay_backdrop_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(
            iced::Color {
                a: 0.5,
                ..theme::bg_base()
            }
            .into(),
        ),
        ..Default::default()
    }
}

fn finder_panel_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(theme::bg_surface().into()),
        border: iced::Border {
            color: theme::border_color(),
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    }
}

fn selected_item_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(theme::accent_dim().scale_alpha(0.2).into()),
        ..Default::default()
    }
}

fn item_style(_theme: &iced::Theme) -> container::Style {
    container::Style::default()
}

fn divider_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(theme::border_color().into()),
        ..Default::default()
    }
}

fn toolbar_bar_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(theme::bg_surface().into()),
        ..Default::default()
    }
}

fn toolbar_btn_style(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Some(theme::bg_hover().into()),
        _ => None,
    };
    button::Style {
        background: bg,
        text_color: theme::text_secondary(),
        border: iced::Border {
            radius: 4.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn finder_input_style(
    _theme: &iced::Theme,
    status: iced::widget::text_input::Status,
) -> iced::widget::text_input::Style {
    use iced::widget::text_input;
    let placeholder = theme::text_muted();
    let value = theme::text_primary();
    let selection = theme::accent_dim().scale_alpha(0.3);
    let background = iced::Background::Color(theme::bg_base());
    let border = iced::Border {
        color: iced::Color::TRANSPARENT,
        width: 0.0,
        radius: iced::border::Radius::default(),
    };
    let base = text_input::Style {
        background,
        border,
        icon: theme::text_muted(),
        placeholder,
        value,
        selection,
    };
    match status {
        text_input::Status::Disabled => text_input::Style {
            value: theme::text_muted(),
            ..base
        },
        _ => base,
    }
}

/// `[pre, matched, post]` rich-text spans — match section in `match_color`.
fn highlight_match_spans<'a>(
    line: &str,
    byte_start: usize,
    byte_end: usize,
    base_color: Color,
    match_color: Color,
) -> Vec<Span<'a, (), Font>> {
    let mut spans: Vec<Span<'a, (), Font>> = Vec::new();
    if byte_start >= byte_end || byte_end > line.len() {
        spans.push(span(line.to_string()).color(base_color));
        return spans;
    }
    if byte_start > 0 {
        spans.push(span(line[..byte_start].to_string()).color(base_color));
    }
    spans.push(
        span(line[byte_start..byte_end].to_string())
            .color(match_color)
            .font(theme::content_font()),
    );
    if byte_end < line.len() {
        spans.push(span(line[byte_end..].to_string()).color(base_color));
    }
    spans
}

// ── Scroll-into-view (chat block at top of viewport) ──────────────────────

/// Stable widget id for the chat block at `idx`. agent_chat wraps each
/// block in a `container().id(...)` so a custom Operation can read its
/// laid-out bounds and scroll the matching block to the top.
pub fn chat_block_widget_id(idx: usize) -> Id {
    Id::from(format!("chat-block-{idx}"))
}

/// Build a Task that scrolls `scrollable_id` so the child container with
/// `block_id` starts at the top of the scrollable's viewport. Reads the
/// laid-out bounds during the next operation pass — no math, no
/// dependency on padding/wrap/collapse state.
pub fn scroll_block_to_top<M: Send + 'static>(
    scrollable_id: impl Into<Id>,
    block_id: impl Into<Id>,
) -> Task<M> {
    let op: ScrollBlockToTop = ScrollBlockToTop {
        scrollable_id: scrollable_id.into(),
        block_id: block_id.into(),
        block_y: None,
        scrollable_y: None,
    };
    iced::advanced::widget::operate(op).discard()
}

struct ScrollBlockToTop {
    scrollable_id: Id,
    block_id: Id,
    block_y: Option<f32>,
    scrollable_y: Option<f32>,
}

impl Operation<()> for ScrollBlockToTop {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<()>)) {
        operate(self);
    }

    fn container(&mut self, id: Option<&Id>, bounds: Rectangle) {
        if id == Some(&self.block_id) {
            self.block_y = Some(bounds.y);
        }
    }

    fn scrollable(
        &mut self,
        id: Option<&Id>,
        bounds: Rectangle,
        _content_bounds: Rectangle,
        _translation: Vector,
        _state: &mut dyn operation::Scrollable,
    ) {
        if id == Some(&self.scrollable_id) {
            self.scrollable_y = Some(bounds.y);
        }
    }

    fn finish(&self) -> operation::Outcome<()> {
        match (self.block_y, self.scrollable_y) {
            (Some(by), Some(sy)) => {
                // iced reports child bounds during `operate` in
                // *untranslated* layout coords — i.e. the absolute layout
                // position of the child, regardless of the scrollable's
                // current scroll offset. So the child's distance from
                // the scrollable's content origin is just `by - sy`, and
                // that's the offset to scroll to. Adding `translation.y`
                // would double-count after the first scroll.
                let target_y = (by - sy).max(0.0);
                operation::Outcome::Chain(Box::new(operation::scrollable::scroll_to(
                    self.scrollable_id.clone(),
                    operation::scrollable::AbsoluteOffset { x: 0.0, y: target_y }.into(),
                )))
            }
            _ => operation::Outcome::None,
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(s: &[&str]) -> Vec<String> {
        s.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn smart_case_lowercase_is_insensitive() {
        let regex = build_regex("hello").unwrap();
        let ls = lines(&["Hello world", "no match", "say HELLO loudly"]);
        let m = compute_matches_in_lines(&ls, &[], &regex, usize::MAX);
        assert_eq!(m.len(), 2);
        assert_eq!(m[0].line, 0);
        assert_eq!(m[1].line, 2);
    }

    #[test]
    fn smart_case_mixed_is_sensitive() {
        let regex = build_regex("Hello").unwrap();
        let ls = lines(&["Hello world", "say HELLO loudly"]);
        let m = compute_matches_in_lines(&ls, &[], &regex, usize::MAX);
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].line, 0);
    }

    #[test]
    fn regex_metachars_work() {
        let regex = build_regex(r"\bfoo\b").unwrap();
        let ls = lines(&["foo bar", "foobar", "x foo y"]);
        let m = compute_matches_in_lines(&ls, &[], &regex, usize::MAX);
        assert_eq!(m.len(), 2);
    }

    #[test]
    fn invalid_regex_returns_error() {
        assert!(build_regex("[unterminated").is_err());
    }

    #[test]
    fn select_wraps() {
        let mut s = FindState {
            query: "x".into(),
            matches: vec![
                FindMatch { line: 0, byte_start: 0, byte_end: 1, line_text: "x".into(), block_idx: None, block_role: None },
                FindMatch { line: 1, byte_start: 0, byte_end: 1, line_text: "x".into(), block_idx: None, block_role: None },
            ],
            current: 0,
        };
        s.select_next();
        assert_eq!(s.current, 1);
        s.select_next();
        assert_eq!(s.current, 0);
        s.select_prev();
        assert_eq!(s.current, 1);
    }

    #[test]
    fn match_count_capped_by_limit() {
        let regex = build_regex("a").unwrap();
        let ls = lines(&["aaa", "aaa", "aaa"]);
        let m = compute_matches_in_lines(&ls, &[], &regex, 5);
        assert_eq!(m.len(), 5);
    }
}
