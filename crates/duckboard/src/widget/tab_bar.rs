//! Tabbed content viewer with pin support and LRU eviction.

use iced::widget::{button, container, row, scrollable, text, Space};
use iced::{Center, Element, Length};

use crate::theme;
use crate::vcs::DiffData;

// ── Content types ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum TabContent {
    Text(String),
    Diff(DiffData),
}

// ── State ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Tab {
    pub id: String,
    pub title: String,
    pub content: TabContent,
    pub pinned: bool,
}

#[derive(Debug, Clone)]
pub struct TabState {
    pub tabs: Vec<Tab>,
    pub active: Option<usize>,
    pub max_tabs: usize,
}

impl Default for TabState {
    fn default() -> Self {
        Self {
            tabs: vec![],
            active: None,
            max_tabs: 10,
        }
    }
}

impl TabState {
    pub fn open(&mut self, id: String, title: String, content: String) {
        if let Some(idx) = self.tabs.iter().position(|t| t.id == id) {
            self.tabs[idx].content = TabContent::Text(content);
            self.active = Some(idx);
            return;
        }
        self.evict_if_needed();
        self.tabs.push(Tab {
            id,
            title,
            content: TabContent::Text(content),
            pinned: false,
        });
        self.active = Some(self.tabs.len() - 1);
    }

    pub fn open_diff(&mut self, id: String, title: String, diff: DiffData) {
        if let Some(idx) = self.tabs.iter().position(|t| t.id == id) {
            self.tabs[idx].content = TabContent::Diff(diff);
            self.active = Some(idx);
            return;
        }
        self.evict_if_needed();
        self.tabs.push(Tab {
            id,
            title,
            content: TabContent::Diff(diff),
            pinned: false,
        });
        self.active = Some(self.tabs.len() - 1);
    }

    pub fn close(&mut self, idx: usize) {
        if idx >= self.tabs.len() {
            return;
        }
        self.remove(idx);
    }

    pub fn select(&mut self, idx: usize) {
        if idx < self.tabs.len() {
            self.active = Some(idx);
        }
    }

    pub fn toggle_pin(&mut self, idx: usize) {
        if let Some(tab) = self.tabs.get_mut(idx) {
            tab.pinned = !tab.pinned;
        }
    }

    pub fn active_tab(&self) -> Option<&Tab> {
        self.active.and_then(|idx| self.tabs.get(idx))
    }

    /// Close a tab by its artifact id. No-op if not found.
    pub fn close_by_id(&mut self, id: &str) {
        if let Some(idx) = self.tabs.iter().position(|t| t.id == id) {
            self.remove(idx);
        }
    }

    /// Update the content of a text tab by its artifact id. No-op if not found
    /// or if the tab holds a diff.
    pub fn refresh_content(&mut self, id: &str, content: String) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
            if matches!(tab.content, TabContent::Text(_)) {
                tab.content = TabContent::Text(content);
            }
        }
    }

    fn evict_if_needed(&mut self) {
        while self.tabs.len() >= self.max_tabs {
            if let Some(idx) = self.oldest_unpinned() {
                self.remove(idx);
            } else {
                break;
            }
        }
    }

    fn oldest_unpinned(&self) -> Option<usize> {
        self.tabs.iter().position(|t| !t.pinned)
    }

    fn remove(&mut self, idx: usize) {
        self.tabs.remove(idx);
        if self.tabs.is_empty() {
            self.active = None;
        } else if let Some(active) = self.active {
            if active == idx {
                self.active = Some(active.min(self.tabs.len() - 1));
            } else if active > idx {
                self.active = Some(active - 1);
            }
        }
    }
}

// ── Views ────────────────────────────────────────────────────────────────────

pub fn view_bar<'a, M: Clone + 'a>(
    state: &'a TabState,
    on_select: impl Fn(usize) -> M + 'a,
    on_close: impl Fn(usize) -> M + 'a,
    on_pin: impl Fn(usize) -> M + 'a,
) -> Element<'a, M> {
    if state.tabs.is_empty() {
        return Space::new().into();
    }

    let mut tabs_row = row![].spacing(1.0).height(34.0);

    for (i, tab) in state.tabs.iter().enumerate() {
        let is_active = state.active == Some(i);
        let tab_style = if is_active {
            theme::tab_active as fn(&iced::Theme, button::Status) -> button::Style
        } else {
            theme::tab_inactive
        };

        let pin_indicator = if tab.pinned {
            text("\u{25cf} ").size(8).color(theme::ACCENT)
        } else {
            text("").size(8)
        };

        let close_btn = button(text("\u{00d7}").size(14).color(theme::TEXT_MUTED))
            .on_press(on_close(i))
            .padding(0.0)
            .style(theme::icon_button);

        let pin_btn = button(pin_indicator)
            .on_press(on_pin(i))
            .padding(0.0)
            .style(theme::icon_button);

        let tab_btn = button(
            row![
                pin_btn,
                text(&tab.title).size(12),
                Space::new().width(theme::SPACING_SM),
                close_btn,
            ]
            .spacing(theme::SPACING_XS)
            .align_y(Center),
        )
        .on_press(on_select(i))
        .padding([theme::SPACING_XS, theme::SPACING_SM])
        .style(tab_style);

        tabs_row = tabs_row.push(tab_btn);
    }

    container(tabs_row)
        .width(Length::Fill)
        .style(theme::elevated)
        .into()
}

pub fn view_content<'a, M: 'a>(state: &'a TabState) -> Element<'a, M> {
    match state.active_tab() {
        Some(tab) => match &tab.content {
            TabContent::Text(s) => scrollable(
                container(
                    text(s)
                        .size(13)
                        .font(iced::Font::MONOSPACE),
                )
                .padding(theme::SPACING_LG)
                .width(Length::Fill),
            )
            .height(Length::Fill)
            .width(Length::Fill)
            .into(),
            TabContent::Diff(diff) => super::diff_view::view(diff),
        },
        None => container(
            text("Select an item to view its contents")
                .size(14)
                .color(theme::TEXT_MUTED),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Center)
        .align_y(Center)
        .into(),
    }
}
