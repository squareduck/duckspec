//! Tabbed content viewer with pin support, LRU eviction, structural views,
//! and a custom text editor with line numbers.

use iced::widget::{button, container, row, text, Space};
use iced::{Center, Element, Length};

use crate::theme;
use crate::vcs::DiffData;
use crate::widget::structural_view::{self, StructuralData};
use crate::widget::text_edit::{self, EditorAction, EditorState};

// ── Content types ───────────────────────────────────────────────────────────

/// What a tab is currently showing.
#[derive(Debug, Clone)]
pub enum TabView {
    /// Editable text file. `structural` is `Some` when this tab can toggle
    /// back to a structural view (i.e. the file is a known artifact).
    Editor {
        editor: EditorState,
        structural: Option<StructuralData>,
    },
    /// Parsed artifact rendered as a navigable structure.
    Structural {
        data: StructuralData,
        source: String,
    },
    /// VCS diff view.
    Diff(DiffData),
}

// ── Messages emitted by tab content ─────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum TabContentMsg {
    EditorAction(EditorAction),
    Structural(structural_view::StructMsg),
}

// ── State ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Tab {
    pub id: String,
    pub title: String,
    pub view: TabView,
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
    /// Open a plain text file (non-artifact) as an editor tab.
    pub fn open(&mut self, id: String, title: String, content: String) {
        if let Some(idx) = self.tabs.iter().position(|t| t.id == id) {
            self.tabs[idx].view = TabView::Editor {
                editor: EditorState::new(&content),
                structural: None,
            };
            self.active = Some(idx);
            return;
        }
        self.evict_if_needed();
        self.tabs.push(Tab {
            id,
            title,
            view: TabView::Editor {
                editor: EditorState::new(&content),
                structural: None,
            },
            pinned: false,
        });
        self.active = Some(self.tabs.len() - 1);
    }

    /// Open a parsed artifact in structural view.
    pub fn open_structural(
        &mut self,
        id: String,
        title: String,
        source: String,
        data: StructuralData,
    ) {
        if let Some(idx) = self.tabs.iter().position(|t| t.id == id) {
            self.tabs[idx].view = TabView::Structural { data, source };
            self.active = Some(idx);
            return;
        }
        self.evict_if_needed();
        self.tabs.push(Tab {
            id,
            title,
            view: TabView::Structural { data, source },
            pinned: false,
        });
        self.active = Some(self.tabs.len() - 1);
    }

    pub fn open_diff(&mut self, id: String, title: String, diff: DiffData) {
        if let Some(idx) = self.tabs.iter().position(|t| t.id == id) {
            self.tabs[idx].view = TabView::Diff(diff);
            self.active = Some(idx);
            return;
        }
        self.evict_if_needed();
        self.tabs.push(Tab {
            id,
            title,
            view: TabView::Diff(diff),
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

    pub fn active_tab_mut(&mut self) -> Option<&mut Tab> {
        self.active.and_then(|idx| self.tabs.get_mut(idx))
    }

    /// Close a tab by its artifact id. No-op if not found.
    pub fn close_by_id(&mut self, id: &str) {
        if let Some(idx) = self.tabs.iter().position(|t| t.id == id) {
            self.remove(idx);
        }
    }

    /// Update the content of a text tab by its artifact id. No-op if not found
    /// or if the tab holds a diff.
    pub fn refresh_content(
        &mut self,
        id: &str,
        new_source: String,
        highlighter: &crate::highlight::SyntaxHighlighter,
    ) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
            match &mut tab.view {
                TabView::Editor { editor, .. } => {
                    *editor = EditorState::new(&new_source);
                    crate::rehighlight(editor, id, highlighter);
                }
                TabView::Structural { source, .. } => {
                    *source = new_source;
                }
                TabView::Diff(_) => {}
            }
        }
    }

    /// Toggle the active tab between structural and editor views.
    /// Returns `true` if a toggle actually happened.
    pub fn toggle_edit_mode(
        &mut self,
        highlighter: &crate::highlight::SyntaxHighlighter,
    ) -> bool {
        let tab = match self.active.and_then(|idx| self.tabs.get_mut(idx)) {
            Some(t) => t,
            None => return false,
        };

        // Take ownership temporarily to restructure.
        let tab_id = tab.id.clone();
        let old_view = std::mem::replace(
            &mut tab.view,
            TabView::Diff(DiffData {
                path: std::path::PathBuf::new(),
                status: crate::vcs::FileStatus::Modified,
                hunks: vec![],
            }),
        );

        match old_view {
            TabView::Structural { data, source } => {
                let mut editor = EditorState::new(&source);
                crate::rehighlight(&mut editor, &tab_id, highlighter);
                tab.view = TabView::Editor {
                    editor,
                    structural: Some(data),
                };
                true
            }
            TabView::Editor {
                structural: Some(data),
                editor,
                ..
            } => {
                tab.view = TabView::Structural {
                    data,
                    source: editor.text(),
                };
                true
            }
            other => {
                tab.view = other;
                false
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

        let mode_indicator = match &tab.view {
            TabView::Structural { .. } => {
                text("\u{25c6} ").size(8).color(theme::STRUCTURAL_HEADING)
            }
            _ => text("").size(8),
        };

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
                mode_indicator,
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

pub fn view_content(state: &TabState) -> Element<'_, TabContentMsg> {
    match state.active_tab() {
        Some(tab) => match &tab.view {
            TabView::Editor { editor, .. } => {
                text_edit::view(editor, TabContentMsg::EditorAction)
            }
            TabView::Structural { data, .. } => {
                structural_view::view(data).map(TabContentMsg::Structural)
            }
            TabView::Diff(diff) => super::diff_view::view(diff),
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
