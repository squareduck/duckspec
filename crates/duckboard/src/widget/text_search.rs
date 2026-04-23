//! Project-wide text search overlay (cmd-shift-F).
//!
//! Modal similar to the file finder, but searches file *contents* via the
//! ripgrep engine (`grep-searcher` + `grep-regex`). Scope is constrained to
//! either `Project` (everything except `duckspec/`), `Duckspec` (only that
//! folder), or `Both`. Enter opens the top match in a file tab; Shift-Enter
//! opens every match as a read-only "search stack" tab.

use std::path::{Path, PathBuf};

use grep_matcher::Matcher;
use grep_regex::{RegexMatcher, RegexMatcherBuilder};
use grep_searcher::{Searcher, Sink, SinkMatch};

use iced::widget::text::Span;
use iced::widget::{Space, button, column, container, rich_text, row, scrollable, span, text, text_input};
use iced::{Center, Color, Element, Font, Length};

use crate::theme;

const MAX_RESULTS: usize = 500;
const MAX_PREVIEW_LEN: usize = 300;
const MAX_VISIBLE: usize = 50;
pub const SEARCH_INPUT_ID: &str = "text-search-input";
/// Name of the folder (relative to project root) that holds duckspec artifacts.
const DUCKSPEC_DIR: &str = "duckspec";

// ── Messages ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Msg {
    Open,
    Close,
    QueryChanged(String),
    ScopeSelected(Scope),
    SelectNext,
    SelectPrev,
    /// Open the selected match in a single file tab.
    ConfirmTop,
    /// Open every match as a read-only "search stack" tab.
    ConfirmStack,
    /// Results from a background search keyed by `query_id`. Older results
    /// than the current `latest_query_id` are discarded by the handler.
    ResultsReady(u64, Vec<SearchHit>),
}

// ── Scope ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Scope {
    Project,
    Duckspec,
    #[default]
    Both,
}

impl Scope {
    fn label(&self) -> &'static str {
        match self {
            Scope::Project => "Project",
            Scope::Duckspec => "Duckspec",
            Scope::Both => "Both",
        }
    }

    fn includes(&self, rel: &Path) -> bool {
        let in_duckspec = rel
            .components()
            .next()
            .map(|c| c.as_os_str() == DUCKSPEC_DIR)
            .unwrap_or(false);
        match self {
            Scope::Both => true,
            Scope::Project => !in_duckspec,
            Scope::Duckspec => in_duckspec,
        }
    }
}

// ── Hit type ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SearchHit {
    /// Project-root-relative path, display-friendly.
    pub rel_path: String,
    pub abs_path: PathBuf,
    /// 0-based line index.
    pub line: usize,
    /// Match column range as BYTE offsets within `line_text`.
    pub byte_start: usize,
    pub byte_end: usize,
    /// Line content (trimmed end; truncated to MAX_PREVIEW_LEN).
    pub line_text: String,
}

// ── State ───────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct TextSearchState {
    pub visible: bool,
    pub query: String,
    pub scope: Scope,
    pub results: Vec<SearchHit>,
    pub selected: u32,
    /// Monotonic id assigned to each search request; result messages are
    /// discarded unless they carry the latest id.
    pub latest_query_id: u64,
    pub searching: bool,
}

impl Default for TextSearchState {
    fn default() -> Self {
        Self {
            visible: false,
            query: String::new(),
            scope: Scope::Both,
            results: Vec::new(),
            selected: 0,
            latest_query_id: 0,
            searching: false,
        }
    }
}

impl TextSearchState {
    pub fn open(&mut self) {
        self.visible = true;
        self.query.clear();
        self.results.clear();
        self.selected = 0;
        self.searching = false;
    }

    pub fn close(&mut self) {
        self.visible = false;
        self.query.clear();
        self.results.clear();
        self.selected = 0;
        self.searching = false;
    }

    pub fn select_next(&mut self) {
        if !self.results.is_empty() {
            let count = self.results.len().min(MAX_VISIBLE) as u32;
            self.selected = (self.selected + 1).min(count.saturating_sub(1));
        }
    }

    pub fn select_prev(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn selected_hit(&self) -> Option<&SearchHit> {
        self.results.get(self.selected as usize)
    }
}

// ── Search engine ───────────────────────────────────────────────────────────

/// Run a project-wide text search. Returns at most `MAX_RESULTS` hits.
/// Meant to be called inside `tokio::task::spawn_blocking` — walking the
/// filesystem and grepping file contents blocks.
pub fn search_blocking(root: PathBuf, query: String, scope: Scope) -> Vec<SearchHit> {
    if query.is_empty() {
        return Vec::new();
    }
    let Some(matcher) = build_matcher(&query) else {
        return Vec::new();
    };

    let mut results: Vec<SearchHit> = Vec::new();
    let walker = ignore::WalkBuilder::new(&root)
        .hidden(true)
        .git_ignore(true)
        .git_global(true)
        .build();

    for entry in walker.flatten() {
        if results.len() >= MAX_RESULTS {
            break;
        }
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }
        let path = entry.path();
        let Ok(rel) = path.strip_prefix(&root) else {
            continue;
        };
        if !scope.includes(rel) {
            continue;
        }

        let rel_str = rel.to_string_lossy().to_string();
        let abs = path.to_path_buf();
        let mut sink = CollectSink {
            rel_path: rel_str,
            abs_path: abs,
            matcher: &matcher,
            out: &mut results,
        };
        let _ = Searcher::new().search_path(&matcher, path, &mut sink);
    }
    results
}

fn build_matcher(query: &str) -> Option<RegexMatcher> {
    RegexMatcherBuilder::new()
        .case_smart(true)
        .build_literals(&[query])
        .ok()
}

struct CollectSink<'a> {
    rel_path: String,
    abs_path: PathBuf,
    matcher: &'a RegexMatcher,
    out: &'a mut Vec<SearchHit>,
}

impl<'a> Sink for CollectSink<'a> {
    type Error = std::io::Error;

    fn matched(&mut self, _searcher: &Searcher, mat: &SinkMatch) -> Result<bool, std::io::Error> {
        if self.out.len() >= MAX_RESULTS {
            return Ok(false);
        }
        let line_num = mat
            .line_number()
            .map(|n| n.saturating_sub(1) as usize)
            .unwrap_or(0);
        let bytes = mat.bytes();
        // Locate the first match within this line's bytes to highlight it.
        let (byte_start, byte_end) = match self.matcher.find(bytes) {
            Ok(Some(m)) => (m.start(), m.end()),
            _ => (0, 0),
        };
        let line_text_raw = String::from_utf8_lossy(bytes).trim_end().to_string();
        let (line_text, (byte_start, byte_end)) =
            truncate_preview(line_text_raw, byte_start, byte_end);
        self.out.push(SearchHit {
            rel_path: self.rel_path.clone(),
            abs_path: self.abs_path.clone(),
            line: line_num,
            byte_start,
            byte_end,
            line_text,
        });
        Ok(self.out.len() < MAX_RESULTS)
    }
}

/// Keep preview lines bounded. If the match sits past the cutoff we pan the
/// window to include it, ellipsizing the leading portion so the highlight
/// stays visible.
fn truncate_preview(line: String, start: usize, end: usize) -> (String, (usize, usize)) {
    if line.len() <= MAX_PREVIEW_LEN {
        return (line, (start, end));
    }
    // Always keep the match in view. Center it in the window when it's long-ish.
    let margin = 40;
    let begin = start.saturating_sub(margin);
    let take = MAX_PREVIEW_LEN.min(line.len() - begin);
    // Snap begin to a char boundary.
    let begin = snap_char_boundary(&line, begin);
    let end_cut = snap_char_boundary(&line, begin + take);
    let sliced = &line[begin..end_cut];
    let prefix = if begin > 0 { "… " } else { "" };
    let suffix = if end_cut < line.len() { " …" } else { "" };
    let shift = prefix.len();
    let new_line = format!("{prefix}{sliced}{suffix}");
    let new_start = (start.saturating_sub(begin)) + shift;
    let new_end = (end.saturating_sub(begin)) + shift;
    // Clamp to new_line length so a range past the cutoff doesn't panic later.
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

// ── View ────────────────────────────────────────────────────────────────────

pub fn view<'a>(state: &'a TextSearchState) -> Element<'a, Msg> {
    // Segmented scope control: three inline segments sharing a single pill
    // background. The active one gets an accent tint; hover on inactive
    // falls back to the usual list-hover tone.
    let segmented = container(
        row![
            scope_segment(Scope::Project, state.scope),
            scope_segment(Scope::Duckspec, state.scope),
            scope_segment(Scope::Both, state.scope),
        ]
        .spacing(2.0),
    )
    .padding(2.0)
    .style(segmented_track_style);

    let scope_bar = container(segmented).padding([theme::SPACING_SM, theme::SPACING_MD]);

    let input = text_input("Search in files…", &state.query)
        .on_input(Msg::QueryChanged)
        .size(theme::font_md())
        .padding([theme::SPACING_SM, theme::SPACING_MD])
        .width(Length::Fill)
        .style(finder_input_style)
        .id(SEARCH_INPUT_ID);

    let hdivider = || container(Space::new().height(1.0).width(Length::Fill)).style(divider_style);

    let mut list = column![].spacing(0.0);
    let visible_count = state.results.len().min(MAX_VISIBLE);
    for (idx, hit) in state.results.iter().take(MAX_VISIBLE).enumerate() {
        let is_selected = idx as u32 == state.selected;
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
        let path_label = format!("{}:{} ", hit.rel_path, hit.line + 1);
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
                    .on_link_click(|_: ()| unreachable!("text search has no links")),
            )
            .padding([theme::SPACING_XS, theme::SPACING_MD])
            .width(Length::Fill)
            .style(row_style),
        );
    }

    let more_hidden = state.results.len().saturating_sub(visible_count);
    let left_text = if state.searching {
        format!("searching… ({} so far)", state.results.len())
    } else if state.query.is_empty() {
        "type to search".to_string()
    } else if more_hidden > 0 {
        format!("{visible_count} results (+{more_hidden} hidden)")
    } else {
        format!("{} results", state.results.len())
    };
    let stack_hint = "⇧⏎ stack".to_string();
    let left = text(left_text)
        .size(theme::font_sm())
        .color(theme::text_muted());
    let hints = text(format!("⏎ open   {stack_hint}"))
        .size(theme::font_sm())
        .color(theme::text_muted());
    let status_bar = container(
        row![left, Space::new().width(Length::Fill), hints].align_y(Center),
    )
    .padding([theme::SPACING_XS, theme::SPACING_MD])
    .width(Length::Fill);

    let panel = container(
        column![
            scope_bar,
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

    container(column![Space::new().height(80.0), panel].align_x(Center))
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Center)
        .style(overlay_backdrop_style)
        .into()
}

fn scope_segment<'a>(this: Scope, active: Scope) -> Element<'a, Msg> {
    let is_active = this == active;
    let style: fn(&iced::Theme, button::Status) -> button::Style = if is_active {
        segmented_active_style
    } else {
        segmented_inactive_style
    };
    button(
        text(this.label())
            .size(theme::font_sm())
            .color(if is_active {
                theme::text_primary()
            } else {
                theme::text_secondary()
            }),
    )
    .on_press(Msg::ScopeSelected(this))
    .padding([2.0, theme::SPACING_MD])
    .style(style)
    .into()
}

fn segmented_track_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(theme::bg_base().into()),
        border: iced::Border {
            color: theme::border_color(),
            width: 1.0,
            radius: 6.0.into(),
        },
        ..Default::default()
    }
}

fn segmented_active_style(_theme: &iced::Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: Some(theme::accent_dim().scale_alpha(0.35).into()),
        text_color: theme::text_primary(),
        border: iced::Border {
            radius: 4.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn segmented_inactive_style(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Some(theme::bg_hover().into()),
        _ => None,
    };
    let text_color = match status {
        button::Status::Hovered => theme::text_primary(),
        _ => theme::text_secondary(),
    };
    button::Style {
        background: bg,
        text_color,
        border: iced::Border {
            radius: 4.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

// ── Styles ──────────────────────────────────────────────────────────────────

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

/// Build `[pre, matched, post]` rich-text spans for a single preview line.
/// `byte_start..byte_end` is the match range into `line`; any overflow is
/// clamped. If the range is zero-width, the whole line is rendered in
/// `base_color`.
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
