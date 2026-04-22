//! Tabbed content viewer with a dedicated preview tab and file tabs.
//!
//! The first tab slot is reserved for artifacts and diffs opened from the list
//! column — clicking a new item replaces it in place. The remaining slots
//! (up to `MAX_FILE_TABS`) hold files opened via the file finder (Ctrl+P),
//! with oldest-first eviction.

use iced::widget::{button, column, container, row, scrollable, svg, text, Space};
use iced::{Center, Element, Length};

use crate::theme;
use crate::vcs::FileStatus;
use crate::widget::text_edit::{self, EditorAction, EditorState};

const ICON_FILE: &[u8] = include_bytes!("../../assets/icon_file.svg");
const ICON_SPEC: &[u8] = include_bytes!("../../assets/icon_spec.svg");
const ICON_DOC: &[u8] = include_bytes!("../../assets/icon_doc.svg");
const ICON_SPEC_DELTA: &[u8] = include_bytes!("../../assets/icon_spec_delta.svg");
const ICON_DOC_DELTA: &[u8] = include_bytes!("../../assets/icon_doc_delta.svg");

fn icon_for_title(title: &str) -> &'static [u8] {
    match title {
        t if t.starts_with("spec.delta") => ICON_SPEC_DELTA,
        t if t.starts_with("spec") => ICON_SPEC,
        t if t.starts_with("doc.delta") => ICON_DOC_DELTA,
        t if t.starts_with("doc") => ICON_DOC,
        _ => ICON_FILE,
    }
}

const MAX_FILE_TABS: usize = 5;

// ── Content types ───────────────────────────────────────────────────────────

/// What a tab is currently showing.
#[derive(Debug, Clone)]
pub enum TabView {
    /// Editable text file.
    Editor { editor: EditorState },
    /// VCS diff view (read-only editor with per-line backgrounds).
    Diff {
        editor: EditorState,
        path: std::path::PathBuf,
        status: FileStatus,
    },
}

// ── Messages emitted by tab content ─────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum TabContentMsg {
    EditorAction(EditorAction),
}

// ── State ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Tab {
    pub id: String,
    pub title: String,
    pub view: TabView,
}

#[derive(Debug, Clone, Default)]
pub struct TabState {
    /// Slot 0: the preview tab (artifacts / diffs). `None` when nothing has
    /// been opened from the list yet.
    pub preview: Option<Tab>,
    /// File tabs opened via the file finder, ordered oldest-first.
    pub file_tabs: Vec<Tab>,
    /// Which tab is active: `Preview` or `File(index)`.
    pub active: ActiveTab,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum ActiveTab {
    #[default]
    Preview,
    File(usize),
}


impl TabState {
    // ── Total tab helpers ────────────────────────────────────────────────

    /// Iterate all tabs (preview first, then file tabs) with their logical
    /// index used in view messages.
    fn all_tabs(&self) -> Vec<(usize, &Tab)> {
        let mut out = Vec::new();
        if let Some(ref t) = self.preview {
            out.push((0, t));
        }
        for (i, t) in self.file_tabs.iter().enumerate() {
            let idx = if self.preview.is_some() { i + 1 } else { i };
            out.push((idx, t));
        }
        out
    }

    fn logical_to_active(&self, idx: usize) -> ActiveTab {
        if self.preview.is_some() {
            if idx == 0 {
                ActiveTab::Preview
            } else {
                ActiveTab::File(idx - 1)
            }
        } else {
            ActiveTab::File(idx)
        }
    }

    // ── Public API ──────────────────────────────────────────────────────

    /// Open an artifact in the preview tab (replaces existing preview).
    pub fn open_preview(&mut self, id: String, title: String, content: String) {
        self.preview = Some(Tab {
            id,
            title,
            view: TabView::Editor {
                editor: EditorState::new(&content),
            },
        });
        self.active = ActiveTab::Preview;
    }

    /// Open a diff in the preview tab.
    pub fn open_diff(&mut self, id: String, title: String, editor: EditorState, path: std::path::PathBuf, status: FileStatus) {
        self.preview = Some(Tab {
            id,
            title,
            view: TabView::Diff { editor, path, status },
        });
        self.active = ActiveTab::Preview;
    }

    /// Open a file tab (from file finder). Reuses existing tab with same id,
    /// or creates a new one (evicting oldest if over limit).
    pub fn open_file(&mut self, id: String, title: String, content: String) {
        if let Some(idx) = self.file_tabs.iter().position(|t| t.id == id) {
            self.file_tabs[idx].view = TabView::Editor {
                editor: EditorState::new(&content),
            };
            self.active = ActiveTab::File(idx);
            return;
        }
        // Evict oldest file tab if at capacity.
        if self.file_tabs.len() >= MAX_FILE_TABS {
            self.file_tabs.remove(0);
            // Fix active index if it pointed into file_tabs.
            if let ActiveTab::File(fi) = self.active {
                if fi > 0 {
                    self.active = ActiveTab::File(fi - 1);
                } else {
                    self.active = ActiveTab::Preview;
                }
            }
        }
        self.file_tabs.push(Tab {
            id,
            title,
            view: TabView::Editor {
                editor: EditorState::new(&content),
            },
        });
        self.active = ActiveTab::File(self.file_tabs.len() - 1);
    }

    pub fn select(&mut self, idx: usize) {
        self.active = self.logical_to_active(idx);
    }

    pub fn close(&mut self, idx: usize) {
        let target = self.logical_to_active(idx);
        match target {
            ActiveTab::Preview => {
                // Preview tab can't be closed.
            }
            ActiveTab::File(fi) => {
                if fi < self.file_tabs.len() {
                    self.file_tabs.remove(fi);
                    // Fix active pointer.
                    match self.active {
                        ActiveTab::File(active_fi) if active_fi == fi => {
                            if self.file_tabs.is_empty() {
                                self.active = ActiveTab::Preview;
                            } else {
                                self.active =
                                    ActiveTab::File(active_fi.min(self.file_tabs.len() - 1));
                            }
                        }
                        ActiveTab::File(active_fi) if active_fi > fi => {
                            self.active = ActiveTab::File(active_fi - 1);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    pub fn active_tab(&self) -> Option<&Tab> {
        match self.active {
            ActiveTab::Preview => self.preview.as_ref(),
            ActiveTab::File(idx) => self.file_tabs.get(idx),
        }
    }

    pub fn active_tab_mut(&mut self) -> Option<&mut Tab> {
        match self.active {
            ActiveTab::Preview => self.preview.as_mut(),
            ActiveTab::File(idx) => self.file_tabs.get_mut(idx),
        }
    }

    /// Close a tab by its id. No-op if not found. Does not close the preview.
    pub fn close_by_id(&mut self, id: &str) {
        if let Some(fi) = self.file_tabs.iter().position(|t| t.id == id) {
            let logical = if self.preview.is_some() { fi + 1 } else { fi };
            self.close(logical);
        }
    }

    /// Update the content of a text tab by its id. Checks both preview and
    /// file tabs. Preserves scroll position and cursor across the rebuild.
    pub fn refresh_content(
        &mut self,
        id: &str,
        new_source: String,
        highlighter: &crate::highlight::SyntaxHighlighter,
    ) {
        let tab = self
            .preview
            .iter_mut()
            .chain(self.file_tabs.iter_mut())
            .find(|t| t.id == id);
        if let Some(tab) = tab
            && let TabView::Editor { editor, .. } = &mut tab.view {
                let mut next = EditorState::new(&new_source);
                carry_view_state(editor, &mut next);
                *editor = next;
                crate::rehighlight(editor, id, highlighter);
            }
    }

    /// Update a diff tab in place. Preserves scroll/cursor and refreshes
    /// the underlying path/status fields. No-op if the tab isn't a diff.
    pub fn refresh_diff(
        &mut self,
        id: &str,
        mut new_editor: EditorState,
        new_path: std::path::PathBuf,
        new_status: FileStatus,
    ) {
        let tab = self
            .preview
            .iter_mut()
            .chain(self.file_tabs.iter_mut())
            .find(|t| t.id == id);
        if let Some(tab) = tab
            && let TabView::Diff { editor, path, status } = &mut tab.view {
                carry_view_state(editor, &mut new_editor);
                *editor = new_editor;
                *path = new_path;
                *status = new_status;
            }
    }
}

/// Copy view-only state (scroll, cursor, selection, bottom-pin) from `prev`
/// to `next`, clamping cursor/anchor to the new line count.
fn carry_view_state(prev: &EditorState, next: &mut EditorState) {
    next.scroll_x = prev.scroll_x;
    next.scroll_y = prev.scroll_y;
    next.pinned_to_bottom = prev.pinned_to_bottom;
    next.cursor = clamp_pos(prev.cursor, &next.lines);
    next.anchor = prev.anchor.map(|p| clamp_pos(p, &next.lines));
}

fn clamp_pos(pos: text_edit::Pos, lines: &[String]) -> text_edit::Pos {
    let max_line = lines.len().saturating_sub(1);
    let line = pos.line.min(max_line);
    let line_len = lines.get(line).map(|s| s.chars().count()).unwrap_or(0);
    text_edit::Pos::new(line, pos.col.min(line_len))
}

// ── Views ────────────────────────────────────────────────────────────────────

pub fn view_bar<'a, M: Clone + 'a>(
    state: &'a TabState,
    on_select: impl Fn(usize) -> M + 'a,
    on_close: impl Fn(usize) -> M + 'a,
) -> Element<'a, M> {
    let all = state.all_tabs();
    if all.is_empty() {
        return Space::new().into();
    }

    let mut tabs_row = row![].spacing(0.0);

    for (i, (logical_idx, tab)) in all.iter().enumerate() {
        let is_active = match state.active {
            ActiveTab::Preview => state.preview.as_ref().map(|p| &p.id) == Some(&tab.id),
            ActiveTab::File(fi) => state.file_tabs.get(fi).map(|t| &t.id) == Some(&tab.id),
        };
        let is_preview = state.logical_to_active(*logical_idx) == ActiveTab::Preview;

        let tab_style = if is_active {
            theme::tab_active as fn(&iced::Theme, button::Status) -> button::Style
        } else {
            theme::tab_inactive
        };

        let mut tab_row = row![
            text(&tab.title).size(theme::font_sm()),
        ]
        .spacing(theme::SPACING_XS)
        .align_y(Center);

        // File tabs get a close button; the preview tab doesn't.
        if !is_preview {
            let close_btn = crate::widget::collapsible::close_button(on_close(*logical_idx));
            tab_row = tab_row.push(Space::new().width(theme::SPACING_SM));
            tab_row = tab_row.push(close_btn);
        }

        // Asymmetric padding: tabs with a close × use less right padding so
        // the × hugs the tab's right edge.
        let pad = if is_preview {
            iced::Padding { top: theme::SPACING_XS, right: theme::SPACING_MD, bottom: theme::SPACING_XS, left: theme::SPACING_MD }
        } else {
            iced::Padding { top: theme::SPACING_XS, right: theme::SPACING_SM, bottom: theme::SPACING_XS, left: theme::SPACING_MD }
        };
        let tab_btn = button(tab_row)
            .on_press(on_select(*logical_idx))
            .padding(pad)
            .style(tab_style);

        if i > 0 {
            tabs_row = tabs_row.push(tab_separator());
        }
        tabs_row = tabs_row.push(tab_btn);
    }
    // Trailing separator caps the row.
    tabs_row = tabs_row.push(tab_separator());

    let tabs_scroll = scrollable(tabs_row)
        .direction(theme::thin_scrollbar_direction_horizontal())
        .style(theme::thin_scrollbar)
        .width(Length::Fill);

    let bar_border = container(Space::new().width(Length::Fill).height(1.0))
        .width(Length::Fill)
        .style(theme::divider);

    column![
        container(tabs_scroll).width(Length::Fill).style(theme::tab_bar),
        bar_border,
    ]
    .into()
}

/// 1px vertical hairline used between adjacent tabs and as a trailing cap.
/// Sized to the tab's natural height (computed the same way as
/// `collapsible::add_button`) so it doesn't stretch the parent row.
fn tab_separator<'a, M: 'a>() -> Element<'a, M> {
    let h = theme::font_sm() * 1.3 + 2.0 * theme::SPACING_XS;
    container(Space::new().width(1.0).height(h))
        .style(theme::divider)
        .into()
}

pub fn view_content(state: &TabState) -> Element<'_, TabContentMsg> {
    match state.active_tab() {
        Some(tab) => {
            // Both diff and non-diff path bars render the same shape: an SVG
            // file-type icon followed by the path. Diff tabs differ only by
            // the icon's tint (status color) — keeping the row construction
            // identical guarantees both bars render at the same height.
            let (icon_bytes, icon_color, path_text) = match &tab.view {
                TabView::Diff { path, status, .. } => (
                    icon_for_title(&tab.title),
                    theme::vcs_status_color(status),
                    path.display().to_string(),
                ),
                _ => {
                    let display = tab.id.strip_prefix("file:").unwrap_or(&tab.id).to_string();
                    (icon_for_title(&tab.title), theme::text_muted(), display)
                }
            };
            let leading: Element<'_, TabContentMsg> = svg(svg::Handle::from_memory(icon_bytes))
                .width(theme::font_sm())
                .height(theme::font_sm())
                .style(theme::svg_tint(icon_color))
                .into();
            let path_row = container(
                row![
                    leading,
                    text(path_text)
                        .size(theme::font_sm())
                        .color(theme::text_secondary()),
                ]
                .spacing(theme::SPACING_XS)
                .align_y(Center),
            )
            .padding([theme::SPACING_XS, theme::SPACING_SM])
            .width(Length::Fill);
            let header: Element<'_, TabContentMsg> = column![
                path_row,
                container(Space::new().width(Length::Fill).height(1.0))
                    .style(theme::divider),
            ]
            .into();

            let body: Element<'_, TabContentMsg> = match &tab.view {
                TabView::Editor { editor, .. } => {
                    text_edit::view(editor, TabContentMsg::EditorAction)
                }
                TabView::Diff { editor, .. } => {
                    text_edit::TextEdit::new(editor, TabContentMsg::EditorAction)
                        .read_only(true)
                        .show_gutter(false)
                        .into()
                }
            };

            column![header, body].height(Length::Fill).into()
        }
        None => container(
            text("Select an item to view its contents")
                .size(theme::font_md())
                .color(theme::text_muted()),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Center)
        .align_y(Center)
        .into(),
    }
}
