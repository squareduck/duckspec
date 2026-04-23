//! File finder overlay — fuzzy file search with nucleo.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use iced::widget::text::Span;
use iced::widget::{Space, column, container, rich_text, scrollable, span, text, text_input};
use iced::{Center, Color, Element, Font, Length};
use nucleo::pattern::{CaseMatching, Normalization};
use nucleo::{Config, Matcher, Nucleo};

use crate::theme;

const MAX_VISIBLE: usize = 15;

// ── Messages ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Msg {
    Open,
    Close,
    QueryChanged(String),
    SelectNext,
    SelectPrev,
    Confirm,
}

// ── State ───────────────────────────────────────────────────────────────────

pub struct FileFinderState {
    pub visible: bool,
    pub query: String,
    pub selected: u32,
    matcher: Option<Nucleo<String>>,
    /// Scratch matcher reused to compute match indices for highlighting.
    /// Wrapped in `RefCell` because `view` only takes a shared reference.
    index_matcher: RefCell<Matcher>,
}

impl Default for FileFinderState {
    fn default() -> Self {
        Self {
            visible: false,
            query: String::new(),
            selected: 0,
            matcher: None,
            index_matcher: RefCell::new(Matcher::default()),
        }
    }
}

// Nucleo<String> is not Debug/Clone, so skip derive for the parent.
impl std::fmt::Debug for FileFinderState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileFinderState")
            .field("visible", &self.visible)
            .field("query", &self.query)
            .field("selected", &self.selected)
            .finish()
    }
}

impl FileFinderState {
    /// Open the file finder, walking project files and injecting them into nucleo.
    pub fn open(&mut self, project_root: &Path) {
        self.visible = true;
        self.query.clear();
        self.selected = 0;

        let mut matcher = Nucleo::new(Config::DEFAULT, Arc::new(|| {}), None, 1);

        let injector = matcher.injector();
        walk_project_files(project_root, |rel_path| {
            let path_str = rel_path.to_string_lossy().to_string();
            injector.push(path_str, |s, cols| {
                cols[0] = s.as_str().into();
            });
        });

        // Initial tick to process all items.
        matcher.tick(10);
        self.matcher = Some(matcher);
    }

    pub fn close(&mut self) {
        self.visible = false;
        self.query.clear();
        self.selected = 0;
        self.matcher = None;
    }

    pub fn set_query(&mut self, query: String) {
        self.query = query;
        if let Some(ref mut matcher) = self.matcher {
            matcher.pattern.reparse(
                0,
                &self.query,
                CaseMatching::Smart,
                Normalization::Smart,
                self.query.starts_with(char::is_lowercase),
            );
            matcher.tick(10);
        }
        self.selected = 0;
    }

    pub fn select_next(&mut self) {
        let count = self.match_count();
        if count > 0 {
            self.selected = (self.selected + 1).min(count - 1);
        }
    }

    pub fn select_prev(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    /// Returns the path of the currently selected match.
    pub fn selected_path(&self) -> Option<PathBuf> {
        let matcher = self.matcher.as_ref()?;
        let snap = matcher.snapshot();
        let item = snap.get_matched_item(self.selected)?;
        Some(PathBuf::from(item.data.as_str()))
    }

    pub fn match_count(&self) -> u32 {
        self.matcher
            .as_ref()
            .map(|m| m.snapshot().matched_item_count())
            .unwrap_or(0)
    }

    /// Collect the visible matched items along with nucleo's char-index
    /// highlight positions (sorted, deduped) and a selected flag.
    fn matched_items(&self) -> Vec<(String, Vec<u32>, bool)> {
        let Some(ref matcher) = self.matcher else {
            return vec![];
        };
        let snap = matcher.snapshot();
        let count = snap.matched_item_count().min(MAX_VISIBLE as u32);
        let mut index_matcher = self.index_matcher.borrow_mut();
        (0..count)
            .filter_map(|i| {
                let item = snap.get_matched_item(i)?;
                let path = item.data.clone();
                let mut indices = Vec::new();
                let _ = snap.pattern().column_pattern(0).indices(
                    item.matcher_columns[0].slice(..),
                    &mut index_matcher,
                    &mut indices,
                );
                indices.sort_unstable();
                indices.dedup();
                Some((path, indices, i == self.selected))
            })
            .collect()
    }
}

// ── File walking ────────────────────────────────────────────────────────────

fn walk_project_files(root: &Path, mut cb: impl FnMut(&Path)) {
    let walker = ignore::WalkBuilder::new(root)
        .hidden(true)
        .git_ignore(true)
        .git_global(true)
        .build();

    for entry in walker.flatten() {
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }
        if let Ok(rel) = entry.path().strip_prefix(root) {
            cb(rel);
        }
    }
}

// ── View ────────────────────────────────────────────────────────────────────

pub fn view<'a>(state: &'a FileFinderState) -> Element<'a, Msg> {
    let input = text_input("Search files...", &state.query)
        .on_input(Msg::QueryChanged)
        .size(theme::font_md())
        .font(theme::content_font())
        .padding([theme::SPACING_SM, theme::SPACING_MD])
        .width(Length::Fill)
        .style(finder_input_style)
        .id("file-finder-input");

    // 1px horizontal rule between the input and the list, matching the panel
    // border color so the input reads as part of the chrome.
    let input_divider =
        container(Space::new().height(1.0).width(Length::Fill)).style(divider_style);

    let mut list = column![].spacing(0.0);
    for (path, match_indices, is_selected) in state.matched_items() {
        let style: fn(&iced::Theme) -> container::Style = if is_selected {
            selected_item_style
        } else {
            item_style
        };
        let base_color = if is_selected {
            theme::text_primary()
        } else {
            theme::text_secondary()
        };
        let spans = highlight_spans(&path, &match_indices, base_color, theme::accent());
        list = list.push(
            container(
                rich_text(spans)
                    .size(theme::font_md())
                    .font(theme::content_font())
                    .on_link_click(|_: ()| unreachable!("file finder has no links")),
            )
            .padding([theme::SPACING_XS, theme::SPACING_MD])
            .width(Length::Fill)
            .style(style),
        );
    }

    let count = state.match_count();
    let total = state
        .matcher
        .as_ref()
        .map(|m| m.snapshot().item_count())
        .unwrap_or(0);
    let status = text(format!("{count} / {total}"))
        .size(theme::font_sm())
        .font(theme::content_font())
        .color(theme::text_muted());

    let panel = container(
        column![
            input,
            input_divider,
            scrollable(list)
                .direction(theme::thin_scrollbar_direction())
                .style(theme::thin_scrollbar)
                .height(Length::Shrink),
            container(status).padding([theme::SPACING_XS, theme::SPACING_MD]),
        ]
        .spacing(0.0)
        .max_width(600.0),
    )
    // 1px all-around padding reserves space for the panel's 1px border so
    // children (selected row, input bg) don't paint over the border edge.
    .padding(1)
    .style(finder_panel_style)
    .max_width(600.0);

    // Center the panel horizontally, place near top.
    container(column![Space::new().height(80.0), panel,].align_x(Center))
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Center)
        .style(overlay_backdrop_style)
        .into()
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

/// Borderless input that blends into the panel: rounded top corners match
/// the panel's outer radius (minus the 1px padding), flat bottom so the
/// input meets the horizontal divider cleanly.
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
        radius: iced::border::Radius::default().top_left(7.0).top_right(7.0),
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

/// Split `path` into rich-text spans so that characters whose char-index
/// appears in `match_indices` are rendered in `match_color` and the rest in
/// `base_color`. `match_indices` must be sorted and deduped.
fn highlight_spans<'a>(
    path: &str,
    match_indices: &[u32],
    base_color: Color,
    match_color: Color,
) -> Vec<Span<'a, (), Font>> {
    let mut spans: Vec<Span<'a, (), Font>> = Vec::new();
    if match_indices.is_empty() {
        spans.push(span(path.to_string()).color(base_color));
        return spans;
    }

    let mut cursor = 0usize;
    let mut buf = String::new();
    let mut matched_run = false;

    let flush = |buf: &mut String, matched: bool, spans: &mut Vec<Span<'a, (), Font>>| {
        if !buf.is_empty() {
            let color = if matched { match_color } else { base_color };
            let s = std::mem::take(buf);
            spans.push(span(s).color(color));
        }
    };

    for (char_idx, ch) in path.chars().enumerate() {
        let is_match = cursor < match_indices.len() && match_indices[cursor] as usize == char_idx;
        if is_match {
            cursor += 1;
        }
        if buf.is_empty() {
            matched_run = is_match;
        } else if is_match != matched_run {
            flush(&mut buf, matched_run, &mut spans);
            matched_run = is_match;
        }
        buf.push(ch);
    }
    flush(&mut buf, matched_run, &mut spans);
    spans
}
