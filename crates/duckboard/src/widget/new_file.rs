//! New-file overlay — pick a project-root-relative path with tab-completion
//! over directory + file candidates, then create (or open if it exists) the
//! target file.
//!
//! Mirrors `project_picker` in input handling (live `read_dir`, tab-completes
//! the last segment, backspace-on-trailing-slash erases the full segment).
//! Differs in three ways:
//!
//! 1. Paths are project-root-relative — no `~` expansion, no absolute paths.
//! 2. Candidates include both directories and files. Dirs sort first (so
//!    Tab biases toward descent); files complete without a trailing slash.
//! 3. Confirmation either opens an existing file or creates a new one. The
//!    actual filesystem write and tab-open live in `main.rs` — this widget
//!    only resolves the typed path.

use std::path::{Path, PathBuf};

use iced::widget::{Space, column, container, row, scrollable, svg, text, text_input};
use iced::{Center, Element, Length};

use crate::theme;

pub const INPUT_ID: &str = "new-file-input";

const ICON_FOLDER: &[u8] = include_bytes!("../../assets/icon_folder.svg");
const ICON_FILE: &[u8] = include_bytes!("../../assets/icon_file.svg");

const MAX_VISIBLE: usize = 15;

// ── Messages ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Msg {
    /// Open with a starting query (typically the relative dir of the focused
    /// editor tab, with a trailing `/`, or an empty string for project root).
    OpenAt(String),
    Close,
    QueryChanged(String),
    SelectNext,
    SelectPrev,
    TabComplete,
    Confirm,
}

// ── Candidate kind ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Dir,
    File,
}

#[derive(Debug, Clone)]
struct Candidate {
    name: String,
    kind: Kind,
}

// ── State ───────────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct NewFileState {
    pub visible: bool,
    pub query: String,
    pub selected: u32,
    /// Project root, captured on `open` so resolution doesn't need it passed
    /// back in on every keystroke. Cleared on `close`.
    project_root: Option<PathBuf>,
    candidates: Vec<Candidate>,
    /// Resolved parent directory of the current query (absolute), or `None`
    /// if the typed path escapes the project root or doesn't resolve.
    parent: Option<PathBuf>,
}

/// Outcome of pressing Enter on the modal — what `main` should do next.
pub enum ConfirmAction {
    /// The typed path resolves to an existing file under project root. Open
    /// it in a tab.
    Open(PathBuf),
    /// The typed path doesn't exist yet under project root. Create the
    /// parent directories (if missing) and the file, then open it.
    Create(PathBuf),
}

impl NewFileState {
    pub fn open_at(&mut self, project_root: &Path, starting: String) {
        self.visible = true;
        self.project_root = Some(project_root.to_path_buf());
        self.set_query(starting);
    }

    pub fn close(&mut self) {
        self.visible = false;
        self.query.clear();
        self.selected = 0;
        self.candidates.clear();
        self.parent = None;
        self.project_root = None;
    }

    pub fn set_query(&mut self, query: String) {
        self.query = query;
        self.selected = 0;
        self.recompute();
    }

    /// Backspace on a trailing `/` erases the full last segment — same UX as
    /// `project_picker::handle_input`. Returns `true` when the segment-erase
    /// path was taken so the caller can snap the caret to the new end.
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

    /// Replace the last path segment with the currently-selected candidate.
    /// Directories get a trailing `/` so further typing descends; files
    /// complete without one so Enter creates/opens them directly.
    pub fn tab_complete(&mut self) {
        let Some(cand) = self.candidates.get(self.selected as usize).cloned() else {
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
        let suffix = match cand.kind {
            Kind::Dir => "/",
            Kind::File => "",
        };
        self.set_query(format!("{base}{}{suffix}", cand.name));
    }

    /// Decide what Enter should do given the current query.
    /// Returns `None` for empty paths, trailing-slash paths (no file to
    /// create), and paths that escape the project root.
    pub fn confirm_action(&self) -> Option<ConfirmAction> {
        if self.query.is_empty() || self.query.ends_with('/') {
            return None;
        }
        let root = self.project_root.as_deref()?;
        let resolved = resolve_under_root(root, &self.query)?;
        if resolved.is_file() {
            Some(ConfirmAction::Open(resolved))
        } else if resolved.exists() {
            // It exists but isn't a file (directory, symlink-to-dir, etc.) —
            // nothing safe to do here. Treat as no-op so Enter doesn't
            // surprise the user with a partial action.
            None
        } else {
            Some(ConfirmAction::Create(resolved))
        }
    }

    fn recompute(&mut self) {
        self.candidates.clear();
        let Some(root) = self.project_root.clone() else {
            self.parent = None;
            return;
        };
        // Treat an empty query as "browse project root."
        let resolved = if self.query.is_empty() {
            Some(root.clone())
        } else {
            resolve_under_root(&root, &self.query)
        };
        let Some(resolved) = resolved else {
            self.parent = None;
            return;
        };

        let (parent, prefix) = if self.query.is_empty() || self.query.ends_with('/') {
            (resolved.clone(), String::new())
        } else {
            match resolved.parent().map(Path::to_path_buf) {
                Some(p) => (
                    p,
                    resolved
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default(),
                ),
                None => (resolved.clone(), String::new()),
            }
        };
        // Guard: the parent must still live inside project root (handles
        // queries like `..` that resolve to the parent of the root).
        if !parent.starts_with(&root) {
            self.parent = None;
            return;
        }
        self.parent = Some(parent.clone());

        let Ok(entries) = std::fs::read_dir(&parent) else {
            return;
        };
        let prefix_lower = prefix.to_lowercase();
        let include_hidden = prefix.starts_with('.');
        let all: Vec<Candidate> = entries
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let ft = e.file_type().ok()?;
                let kind = if ft.is_dir() {
                    Kind::Dir
                } else if ft.is_file() {
                    Kind::File
                } else {
                    return None;
                };
                let name = e.file_name().to_string_lossy().to_string();
                if !include_hidden && name.starts_with('.') {
                    return None;
                }
                Some(Candidate { name, kind })
            })
            .collect();

        // Rank: dirs first (so Tab biases toward descent), then prefix
        // matches over subsequence matches, then alphabetical.
        let mut scored: Vec<(u8, u8, usize, String, Kind)> = all
            .into_iter()
            .filter_map(|c| {
                let dir_rank: u8 = match c.kind {
                    Kind::Dir => 0,
                    Kind::File => 1,
                };
                if prefix.is_empty() {
                    return Some((dir_rank, 0, c.name.len(), c.name, c.kind));
                }
                let lower = c.name.to_lowercase();
                if lower.starts_with(&prefix_lower) {
                    Some((dir_rank, 0, c.name.len(), c.name, c.kind))
                } else {
                    subsequence_span(&lower, &prefix_lower)
                        .map(|span| (dir_rank, 1, span, c.name, c.kind))
                }
            })
            .collect();
        scored.sort_by(|a, b| {
            a.0.cmp(&b.0)
                .then(a.1.cmp(&b.1))
                .then(a.2.cmp(&b.2))
                .then_with(|| a.3.to_lowercase().cmp(&b.3.to_lowercase()))
        });
        self.candidates = scored
            .into_iter()
            .map(|(_, _, _, name, kind)| Candidate { name, kind })
            .collect();
    }
}

// ── Path helpers ────────────────────────────────────────────────────────────

/// Resolve `input` (project-root-relative) against `root`, normalizing `..`
/// without touching the filesystem. Returns `None` if the path escapes
/// the root.
fn resolve_under_root(root: &Path, input: &str) -> Option<PathBuf> {
    let mut out = root.to_path_buf();
    for component in Path::new(input).components() {
        use std::path::Component;
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if out == root {
                    return None;
                }
                if !out.pop() {
                    return None;
                }
            }
            Component::Normal(seg) => out.push(seg),
            // Reject absolute / prefix components — relative-only.
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    if !out.starts_with(root) {
        return None;
    }
    Some(out)
}

fn split_last_segment(input: &str) -> (&str, &str) {
    match input.rfind('/') {
        Some(idx) => (&input[..=idx], &input[idx + 1..]),
        None => ("", input),
    }
}

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

pub fn view<'a>(state: &'a NewFileState) -> Element<'a, Msg> {
    let input = text_input("New file path...", &state.query)
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
    for (i, cand) in state.candidates.iter().take(MAX_VISIBLE).enumerate() {
        let is_selected = i as u32 == state.selected;
        let row_style: fn(&iced::Theme) -> container::Style = if is_selected {
            selected_item_style
        } else {
            item_style
        };
        let color = if is_selected {
            theme::text_primary()
        } else {
            theme::text_secondary()
        };
        let (icon_bytes, label) = match cand.kind {
            Kind::Dir => (ICON_FOLDER, format!("{}/", cand.name)),
            Kind::File => (ICON_FILE, cand.name.clone()),
        };
        let icon = svg(svg::Handle::from_memory(icon_bytes))
            .width(theme::font_md())
            .height(theme::font_md())
            .style(theme::svg_tint(theme::text_muted()));
        list = list.push(
            container(
                row![
                    icon,
                    text(label)
                        .size(theme::font_md())
                        .font(theme::content_font())
                        .color(color),
                ]
                .spacing(theme::SPACING_SM)
                .align_y(Center),
            )
            .padding([theme::SPACING_XS, theme::SPACING_MD])
            .width(Length::Fill)
            .style(row_style),
        );
    }

    let status_text = match (&state.parent, state.confirm_action()) {
        (_, Some(ConfirmAction::Open(_))) => "press \u{21B5} to open".to_string(),
        (_, Some(ConfirmAction::Create(_))) => "press \u{21B5} to create".to_string(),
        (Some(p), None) => {
            let total = state.candidates.len();
            format!(
                "{}  \u{00b7}  {} entr{}",
                p.display(),
                total,
                if total == 1 { "y" } else { "ies" }
            )
        }
        (None, None) => "(path escapes project root)".to_string(),
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
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    /// Process-unique temp dir name suffix. Two tests in the same nanosecond
    /// would otherwise collide on the per-test `setup_tree`.
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let counter = COUNTER.fetch_add(1, Ordering::SeqCst);
            let mut p = std::env::temp_dir();
            p.push(format!("duckboard-new-file-test-{nanos}-{counter}"));
            std::fs::create_dir_all(&p).unwrap();
            Self(p)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn setup_tree() -> TempDir {
        let dir = TempDir::new();
        let root = dir.path();
        fs::create_dir_all(root.join("src/widget")).unwrap();
        fs::create_dir_all(root.join("tests")).unwrap();
        fs::write(root.join("Cargo.toml"), "").unwrap();
        fs::write(root.join("src/main.rs"), "").unwrap();
        fs::write(root.join("src/widget/foo.rs"), "").unwrap();
        dir
    }

    #[test]
    fn empty_query_lists_root_entries() {
        let tmp = setup_tree();
        let mut s = NewFileState::default();
        s.open_at(tmp.path(), String::new());
        let names: Vec<&str> = s.candidates.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"src"));
        assert!(names.contains(&"Cargo.toml"));
    }

    #[test]
    fn dirs_sort_before_files() {
        let tmp = setup_tree();
        let mut s = NewFileState::default();
        s.open_at(tmp.path(), String::new());
        let kinds: Vec<Kind> = s.candidates.iter().map(|c| c.kind).collect();
        let first_dir = kinds.iter().position(|k| *k == Kind::Dir);
        let first_file = kinds.iter().position(|k| *k == Kind::File);
        assert!(first_dir.unwrap() < first_file.unwrap());
    }

    #[test]
    fn tab_complete_dir_appends_slash() {
        let tmp = setup_tree();
        let mut s = NewFileState::default();
        s.open_at(tmp.path(), "sr".into());
        s.tab_complete();
        assert_eq!(s.query, "src/");
    }

    #[test]
    fn tab_complete_file_omits_slash() {
        let tmp = setup_tree();
        let mut s = NewFileState::default();
        s.open_at(tmp.path(), "Carg".into());
        s.tab_complete();
        assert_eq!(s.query, "Cargo.toml");
    }

    #[test]
    fn backspace_trailing_slash_erases_segment() {
        let tmp = setup_tree();
        let mut s = NewFileState::default();
        s.open_at(tmp.path(), "src/widget/".into());
        let replaced = s.handle_input("src/widget".into());
        assert!(replaced);
        assert_eq!(s.query, "src/");
    }

    #[test]
    fn confirm_existing_file_opens() {
        let tmp = setup_tree();
        let mut s = NewFileState::default();
        s.open_at(tmp.path(), "Cargo.toml".into());
        match s.confirm_action() {
            Some(ConfirmAction::Open(p)) => assert_eq!(p, tmp.path().join("Cargo.toml")),
            _ => panic!("expected Open"),
        }
    }

    #[test]
    fn confirm_new_file_creates() {
        let tmp = setup_tree();
        let mut s = NewFileState::default();
        s.open_at(tmp.path(), "src/widget/new_thing.rs".into());
        match s.confirm_action() {
            Some(ConfirmAction::Create(p)) => {
                assert_eq!(p, tmp.path().join("src/widget/new_thing.rs"))
            }
            _ => panic!("expected Create"),
        }
    }

    #[test]
    fn confirm_trailing_slash_is_noop() {
        let tmp = setup_tree();
        let mut s = NewFileState::default();
        s.open_at(tmp.path(), "src/".into());
        assert!(s.confirm_action().is_none());
    }

    #[test]
    fn confirm_empty_is_noop() {
        let tmp = setup_tree();
        let mut s = NewFileState::default();
        s.open_at(tmp.path(), String::new());
        assert!(s.confirm_action().is_none());
    }

    #[test]
    fn confirm_existing_dir_is_noop() {
        let tmp = setup_tree();
        let mut s = NewFileState::default();
        s.open_at(tmp.path(), "src".into());
        assert!(s.confirm_action().is_none());
    }

    #[test]
    fn parent_escape_rejected() {
        let tmp = setup_tree();
        let mut s = NewFileState::default();
        s.open_at(tmp.path(), "../escape.txt".into());
        assert!(s.confirm_action().is_none());
    }
}
