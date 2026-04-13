//! File finder overlay — fuzzy file search with nucleo.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use iced::widget::{column, container, scrollable, text, text_input, Space};
use iced::{Center, Element, Length};
use nucleo::pattern::{CaseMatching, Normalization};
use nucleo::{Config, Nucleo};

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
}

impl Default for FileFinderState {
    fn default() -> Self {
        Self {
            visible: false,
            query: String::new(),
            selected: 0,
            matcher: None,
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

        let mut matcher = Nucleo::new(
            Config::DEFAULT,
            Arc::new(|| {}),
            None,
            1,
        );

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

    fn matched_items(&self) -> Vec<(String, bool)> {
        let Some(ref matcher) = self.matcher else {
            return vec![];
        };
        let snap = matcher.snapshot();
        let count = snap.matched_item_count().min(MAX_VISIBLE as u32);
        (0..count)
            .filter_map(|i| {
                let item = snap.get_matched_item(i)?;
                Some((item.data.clone(), i == self.selected))
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
        .size(theme::FONT_MD)
        .padding([theme::SPACING_SM, theme::SPACING_MD])
        .width(Length::Fill)
        .id("file-finder-input");

    let mut list = column![].spacing(0.0);
    for (path, is_selected) in state.matched_items() {
        let style: fn(&iced::Theme) -> container::Style = if is_selected {
            selected_item_style
        } else {
            item_style
        };
        let text_color = if is_selected {
            theme::TEXT_PRIMARY
        } else {
            theme::TEXT_SECONDARY
        };
        list = list.push(
            container(
                text(path)
                    .size(theme::FONT_MD)
                    .font(iced::Font::MONOSPACE)
                    .color(text_color),
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
        .size(theme::FONT_SM)
        .color(theme::TEXT_MUTED);

    let panel = container(
        column![
            input,
            scrollable(list).height(Length::Shrink),
            container(status).padding([theme::SPACING_XS, theme::SPACING_MD]),
        ]
        .spacing(0.0)
        .max_width(600.0),
    )
    .style(finder_panel_style)
    .max_width(600.0);

    // Center the panel horizontally, place near top.
    container(
        column![
            Space::new().height(80.0),
            panel,
        ]
        .align_x(Center),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .align_x(Center)
    .style(overlay_backdrop_style)
    .into()
}

// ── Styles ──────────────────────────────────────────────────────────────────

fn overlay_backdrop_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(iced::Color { a: 0.5, ..theme::BG_BASE }.into()),
        ..Default::default()
    }
}

fn finder_panel_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(theme::BG_SURFACE.into()),
        border: iced::Border {
            color: theme::BORDER_COLOR,
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    }
}

fn selected_item_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(theme::ACCENT_DIM.scale_alpha(0.2).into()),
        ..Default::default()
    }
}

fn item_style(_theme: &iced::Theme) -> container::Style {
    container::Style::default()
}
