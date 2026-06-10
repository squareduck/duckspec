//! Project picker overlay — choose a directory via path input with
//! tab-completion over its directory candidates.
//!
//! Unlike the file-finder, this widget reads the filesystem live on every
//! query change (fine — a single `read_dir` on one directory is cheap and
//! the panel is only open briefly). The query is an editable path with `~`
//! expansion; the candidate list shows directory children of the resolved
//! parent whose names start with the last path segment (case-insensitive).

use std::path::{Path, PathBuf};

use iced::widget::{Space, column, container, row, scrollable, svg, text, text_input};
use iced::{Center, Element, Length};

use crate::theme;

pub const INPUT_ID: &str = "project-picker-input";

const ICON_FOLDER: &[u8] = include_bytes!("../../assets/icon_folder.svg");
const ICON_DOT: &[u8] = include_bytes!("../../assets/icon_dot.svg");

const MAX_VISIBLE: usize = 15;

// ── Messages ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Msg {
    Open,
    Close,
    QueryChanged(String),
    SelectNext,
    SelectPrev,
    TabComplete,
    Confirm,
    /// Pick a specific path (recent entry clicked in the modal).
    PickPath(PathBuf),
    HoverRecent(PathBuf),
    UnhoverRecent(PathBuf),
    /// One-click drop from the recents list. Reversible.
    ForgetRecent(PathBuf),
    /// First click on "Delete data" — arms the destructive follow-up.
    ArmDeleteRecent(PathBuf),
    /// Wipe the per-project data directory and drop from recents.
    DeleteRecentData(PathBuf),
}

// ── State ───────────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct ProjectPickerState {
    pub visible: bool,
    pub query: String,
    pub selected: u32,
    candidates: Vec<String>,
    /// Resolved parent directory of the current query, or `None` if it
    /// doesn't point to a readable directory.
    parent: Option<PathBuf>,
    /// Path of the recent-projects row currently under the cursor.
    pub hovered_recent: Option<PathBuf>,
    /// Recent-projects path armed for destructive delete; cleared on
    /// hover-leave, `close`, or a successful delete.
    pub armed_delete_recent: Option<PathBuf>,
}

impl ProjectPickerState {
    pub fn open(&mut self) {
        self.visible = true;
        // Default to the user's home dir, with a trailing slash so the
        // candidate list shows its children immediately.
        let home = dirs::home_dir()
            .map(|p| format!("{}/", p.display()))
            .unwrap_or_else(|| String::from("/"));
        self.set_query(home);
    }

    pub fn close(&mut self) {
        self.visible = false;
        self.query.clear();
        self.selected = 0;
        self.candidates.clear();
        self.parent = None;
        self.hovered_recent = None;
        self.armed_delete_recent = None;
    }

    pub fn set_query(&mut self, query: String) {
        self.query = query;
        self.selected = 0;
        self.recompute();
    }

    /// When `new_query` removed exactly the trailing `/` from the old query,
    /// erase the full last segment instead — this turns an otherwise-cheap
    /// single-char backspace into "undo the last tab-completion".
    ///
    /// Returns `true` when it took the segment-erase path so the caller
    /// knows to move the cursor to the end.
    pub fn handle_input(&mut self, new_query: String) -> bool {
        let old = &self.query;
        let just_deleted_trailing_slash = new_query.len() + 1 == old.len()
            && old.ends_with('/')
            && old.starts_with(new_query.as_str());
        if just_deleted_trailing_slash {
            let trimmed = old.trim_end_matches('/');
            let stripped = match trimmed.rfind('/') {
                Some(idx) => trimmed[..=idx].to_string(),
                None => String::new(),
            };
            self.set_query(stripped);
            true
        } else {
            self.set_query(new_query);
            false
        }
    }

    pub fn select_next(&mut self) {
        let count = self.candidates.len() as u32;
        if count > 0 {
            self.selected = (self.selected + 1).min(count - 1);
        }
    }

    pub fn select_prev(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    /// Replace the last path segment with the currently-selected candidate
    /// and append a trailing slash so further typing descends into it.
    pub fn tab_complete(&mut self) {
        let Some(name) = self.candidates.get(self.selected as usize).cloned() else {
            return;
        };
        let (parent_str, _) = split_last_segment(&self.query);
        let base = if parent_str.is_empty() {
            String::new()
        } else if parent_str.ends_with('/') {
            parent_str.to_string()
        } else {
            format!("{parent_str}/")
        };
        self.set_query(format!("{base}{name}/"));
    }

    /// The fully-expanded path for the current query, or `None` if the path
    /// doesn't resolve to an existing directory. Used on `Confirm`.
    pub fn resolved_path(&self) -> Option<PathBuf> {
        let expanded = expand_tilde(&self.query)?;
        if expanded.is_dir() {
            Some(expanded)
        } else {
            None
        }
    }

    fn recompute(&mut self) {
        self.candidates.clear();
        let Some(expanded) = expand_tilde(&self.query) else {
            self.parent = None;
            return;
        };

        let (parent, prefix) = if self.query.ends_with('/') {
            (expanded.clone(), String::new())
        } else {
            match expanded.parent().map(Path::to_path_buf) {
                Some(p) => (
                    p,
                    expanded
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default(),
                ),
                None => (expanded.clone(), String::new()),
            }
        };
        self.parent = Some(parent.clone());

        let Ok(entries) = std::fs::read_dir(&parent) else {
            return;
        };
        let prefix_lower = prefix.to_lowercase();
        let include_hidden = prefix.starts_with('.');
        let all_names: Vec<String> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|name| include_hidden || !name.starts_with('.'))
            .collect();

        self.candidates = if prefix.is_empty() {
            let mut names = all_names;
            names.sort_by_key(|a| a.to_lowercase());
            names
        } else {
            // Rank by: exact prefix match first (lower is better), then
            // subsequence score (span between first and last match,
            // shorter is tighter), then name length, then alphabetical.
            let mut scored: Vec<(u8, usize, usize, String)> = all_names
                .into_iter()
                .filter_map(|name| {
                    let lower = name.to_lowercase();
                    if lower.starts_with(&prefix_lower) {
                        Some((0, 0, name.len(), name))
                    } else {
                        subsequence_span(&lower, &prefix_lower)
                            .map(|span| (1, span, name.len(), name))
                    }
                })
                .collect();
            scored.sort_by(|a, b| {
                a.0.cmp(&b.0)
                    .then(a.1.cmp(&b.1))
                    .then(a.2.cmp(&b.2))
                    .then_with(|| a.3.to_lowercase().cmp(&b.3.to_lowercase()))
            });
            scored.into_iter().map(|(_, _, _, n)| n).collect()
        };
    }
}

// ── Path helpers ────────────────────────────────────────────────────────────

/// Expand a leading `~` or `~/` in `input` to the current user's home dir.
/// Returns the resulting path; no filesystem access.
fn expand_tilde(input: &str) -> Option<PathBuf> {
    if input.is_empty() {
        return dirs::home_dir();
    }
    if input == "~" {
        return dirs::home_dir();
    }
    if let Some(rest) = input.strip_prefix("~/") {
        let mut home = dirs::home_dir()?;
        home.push(rest);
        return Some(home);
    }
    Some(PathBuf::from(input))
}

/// Split `input` into (everything-up-to-and-including-the-last-slash, last-segment).
/// If there's no slash, the parent string is empty.
fn split_last_segment(input: &str) -> (&str, &str) {
    match input.rfind('/') {
        Some(idx) => (&input[..=idx], &input[idx + 1..]),
        None => ("", input),
    }
}

/// If every char of `needle` appears in `haystack` in order, return the
/// span (last_idx - first_idx + 1) — a tight match scores lower. Both
/// inputs must already be lowercased by the caller.
fn subsequence_span(haystack: &str, needle: &str) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    let mut needle_chars = needle.chars();
    let mut want = needle_chars.next()?;
    let mut first: Option<usize> = None;
    for (idx, ch) in haystack.chars().enumerate() {
        if ch == want {
            if first.is_none() {
                first = Some(idx);
            }
            match needle_chars.next() {
                Some(next) => want = next,
                None => return Some(idx - first.unwrap() + 1),
            }
        }
    }
    None
}

// ── View ────────────────────────────────────────────────────────────────────

pub fn view<'a>(state: &'a ProjectPickerState, recent: &'a [PathBuf]) -> Element<'a, Msg> {
    let input = text_input("Path to project...", &state.query)
        .on_input(Msg::QueryChanged)
        .on_submit(Msg::Confirm)
        .size(theme::font_md())
        .font(theme::content_font())
        .padding([theme::SPACING_SM, theme::SPACING_MD])
        .width(Length::Fill)
        .style(finder_input_style)
        .id(INPUT_ID);

    let input_divider =
        container(Space::new().height(1.0).width(Length::Fill)).style(divider_style);

    let mut list = column![].spacing(0.0);

    // Recent projects appear at the top of the list so users can jump back
    // with a single click, even if the query still points at the default
    // home dir. Limit to 5 to keep the panel compact.
    if !recent.is_empty() {
        list = list.push(
            container(
                text("Recent")
                    .size(theme::font_sm())
                    .font(theme::content_font())
                    .color(theme::text_secondary()),
            )
            .padding([theme::SPACING_XS, theme::SPACING_MD]),
        );
        for path in recent.iter().take(5) {
            list = list.push(recent_row(state, path));
        }
        list =
            list.push(container(Space::new().height(1.0).width(Length::Fill)).style(divider_style));
        list = list.push(
            container(
                text("Browse")
                    .size(theme::font_sm())
                    .font(theme::content_font())
                    .color(theme::text_secondary()),
            )
            .padding([theme::SPACING_XS, theme::SPACING_MD]),
        );
    }

    for (i, name) in state.candidates.iter().take(MAX_VISIBLE).enumerate() {
        let is_selected = i as u32 == state.selected;
        let style: fn(&iced::Theme) -> container::Style = if is_selected {
            selected_item_style
        } else {
            item_style
        };
        let color = if is_selected {
            theme::text_primary()
        } else {
            theme::text_secondary()
        };
        let icon = svg(svg::Handle::from_memory(ICON_FOLDER))
            .width(theme::font_md())
            .height(theme::font_md())
            .style(theme::svg_tint(theme::text_muted()));
        list = list.push(
            container(
                row![
                    icon,
                    text(format!("{name}/"))
                        .size(theme::font_md())
                        .font(theme::content_font())
                        .color(color),
                ]
                .spacing(theme::SPACING_SM)
                .align_y(Center),
            )
            .padding([theme::SPACING_XS, theme::SPACING_MD])
            .width(Length::Fill)
            .style(style),
        );
    }

    let total = state.candidates.len();
    let status_text = match &state.parent {
        Some(p) => format!(
            "{}  \u{00b7}  {} match{}",
            p.display(),
            total,
            if total == 1 { "" } else { "es" }
        ),
        None => "(path does not resolve)".to_string(),
    };
    let status = text(status_text)
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
    .padding(1)
    .style(finder_panel_style)
    .max_width(600.0);

    container(column![Space::new().height(80.0), panel].align_x(Center))
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Center)
        .style(overlay_backdrop_style)
        .into()
}

/// One row in the "Recent" section: the path button plus hover-revealed
/// "Forget" / "Delete data" actions. Mirrors the dashboard's recent-row
/// behavior so the two surfaces stay in lockstep.
fn recent_row<'a>(state: &'a ProjectPickerState, path: &'a Path) -> Element<'a, Msg> {
    use iced::widget::{button, mouse_area};
    let label = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string());
    let full = path.display().to_string();
    let dot = svg(svg::Handle::from_memory(ICON_DOT))
        .width(theme::font_sm())
        .height(theme::font_sm())
        .style(theme::svg_tint(theme::text_muted()));

    let is_hovered = state.hovered_recent.as_deref() == Some(path);
    let is_armed_delete = state.armed_delete_recent.as_deref() == Some(path);

    // Transparent click target; row background is drawn by the outer
    // container so the actions area sits inside the same hover band.
    let open_btn = button(
        row![
            dot,
            column![
                text(label)
                    .size(theme::font_md())
                    .font(theme::content_font())
                    .color(theme::text_primary()),
                text(full)
                    .size(theme::font_sm())
                    .font(theme::content_font())
                    .color(theme::text_muted()),
            ]
            .spacing(0.0),
        ]
        .spacing(theme::SPACING_SM)
        .align_y(Center),
    )
    .on_press(Msg::PickPath(path.to_path_buf()))
    .width(Length::Fill)
    .padding([theme::SPACING_XS, theme::SPACING_MD])
    .style(transparent_row_button);

    let actions: Element<'a, Msg> = if is_hovered {
        let forget = button(text("Forget").size(theme::font_sm()))
            .on_press(Msg::ForgetRecent(path.to_path_buf()))
            .padding([theme::SPACING_XS, theme::SPACING_SM])
            .style(theme::icon_button);
        let (delete_label, delete_msg, delete_style) = if is_armed_delete {
            (
                "Delete data — click again",
                Msg::DeleteRecentData(path.to_path_buf()),
                destructive_button_armed
                    as fn(
                        &iced::Theme,
                        iced::widget::button::Status,
                    ) -> iced::widget::button::Style,
            )
        } else {
            (
                "Delete data",
                Msg::ArmDeleteRecent(path.to_path_buf()),
                theme::icon_button
                    as fn(
                        &iced::Theme,
                        iced::widget::button::Status,
                    ) -> iced::widget::button::Style,
            )
        };
        let delete = button(text(delete_label).size(theme::font_sm()))
            .on_press(delete_msg)
            .padding([theme::SPACING_XS, theme::SPACING_SM])
            .style(delete_style);
        row![forget, delete]
            .spacing(theme::SPACING_XS)
            .align_y(Center)
            .into()
    } else {
        Space::new().into()
    };

    let row_content = row![open_btn, actions]
        .spacing(0.0)
        .align_y(Center)
        .width(Length::Fill);

    let highlighted = container(row_content)
        .padding(iced::Padding {
            top: 0.0,
            right: theme::SPACING_SM,
            bottom: 0.0,
            left: 0.0,
        })
        .style(if is_hovered {
            recent_row_hover_bg as fn(&iced::Theme) -> container::Style
        } else {
            recent_row_idle_bg
        });

    mouse_area(highlighted)
        .on_enter(Msg::HoverRecent(path.to_path_buf()))
        .on_exit(Msg::UnhoverRecent(path.to_path_buf()))
        .into()
}

fn transparent_row_button(
    _theme: &iced::Theme,
    _status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    iced::widget::button::Style {
        background: None,
        text_color: theme::text_primary(),
        border: iced::Border::default(),
        ..Default::default()
    }
}

fn destructive_button_armed(
    _theme: &iced::Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let bg = match status {
        iced::widget::button::Status::Hovered => Some(theme::bg_list_hover().into()),
        _ => None,
    };
    iced::widget::button::Style {
        background: bg,
        text_color: theme::error(),
        border: iced::Border::default(),
        ..Default::default()
    }
}

fn recent_row_hover_bg(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(theme::bg_list_hover().into()),
        ..Default::default()
    }
}

fn recent_row_idle_bg(_theme: &iced::Theme) -> container::Style {
    container::Style::default()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_last_segment_with_trailing_slash() {
        assert_eq!(split_last_segment("/a/b/c/"), ("/a/b/c/", ""));
    }

    #[test]
    fn splits_last_segment_mid_word() {
        assert_eq!(split_last_segment("/a/b/c"), ("/a/b/", "c"));
    }

    #[test]
    fn splits_last_segment_no_slash() {
        assert_eq!(split_last_segment("foo"), ("", "foo"));
    }

    #[test]
    fn backspace_trailing_slash_erases_segment() {
        let mut s = ProjectPickerState::default();
        s.set_query("/Users/alice/Dev/".into());
        // Simulate text_input reporting "one char removed from the end".
        let replaced = s.handle_input("/Users/alice/Dev".into());
        assert!(replaced);
        assert_eq!(s.query, "/Users/alice/");
    }

    #[test]
    fn backspace_mid_string_unchanged() {
        let mut s = ProjectPickerState::default();
        s.set_query("/Users/alice".into());
        let replaced = s.handle_input("/Users/alie".into());
        assert!(!replaced);
        assert_eq!(s.query, "/Users/alie");
    }

    #[test]
    fn subsequence_tight_match_scores_lower() {
        // "dsp" is tight in "dspec" (span 3), looser in "duckspec" (6).
        let a = subsequence_span("dspec", "dsp").unwrap();
        let b = subsequence_span("duckspec", "dsp").unwrap();
        assert!(a < b);
    }

    #[test]
    fn subsequence_missing_chars_none() {
        assert!(subsequence_span("abcdef", "xy").is_none());
    }

    #[test]
    fn subsequence_empty_needle_zero() {
        assert_eq!(subsequence_span("anything", ""), Some(0));
    }

    #[test]
    fn backspace_root_slash_clears() {
        let mut s = ProjectPickerState::default();
        s.set_query("/".into());
        let replaced = s.handle_input("".into());
        assert!(replaced);
        assert_eq!(s.query, "");
    }
}
