//! Change area — single change workspace with three-column layout.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use iced::widget::{Space, button, column, container, row, text};
use iced::{Element, Length};

use crate::data::{ChangeData, ProjectData, StepCompletion};
use crate::theme;
use crate::vcs::{ChangedFile, FileStatus};
use crate::widget::list_view::{self, Badge, ListRow};
use crate::widget::{collapsible, interaction_toggle, tab_bar, tree_view, vertical_scroll};

use super::interaction::{self, AgentSession, InteractionMode, InteractionState, SessionControls};

const ICON_BRANCH: &[u8] = include_bytes!("../../assets/icon_branch.svg");
const ICON_FILE: &[u8] = include_bytes!("../../assets/icon_file.svg");
const ICON_SPEC: &[u8] = include_bytes!("../../assets/icon_spec.svg");
const ICON_DOC: &[u8] = include_bytes!("../../assets/icon_doc.svg");
const ICON_SPEC_DELTA: &[u8] = include_bytes!("../../assets/icon_spec_delta.svg");
const ICON_DOC_DELTA: &[u8] = include_bytes!("../../assets/icon_doc_delta.svg");
const ICON_STEP: &[u8] = include_bytes!("../../assets/icon_step.svg");
const ICON_STEP_DONE: &[u8] = include_bytes!("../../assets/icon_step_done.svg");
const ICON_STEP_PARTIAL: &[u8] = include_bytes!("../../assets/icon_step_partial.svg");
const ICON_EXPLORE: &[u8] = include_bytes!("../../assets/icon_explore.svg");

// ── State ────────────────────────────────────────────────────────────────────

pub struct State {
    pub selected_change: Option<String>,
    pub expanded_sections: HashSet<String>,
    pub expanded_nodes: HashSet<String>,
    /// Directory paths (repo-relative, as display strings) expanded in the
    /// changed-files tree.
    pub expanded_file_dirs: HashSet<String>,
    /// Directory paths previously surfaced by `set_changed_files`. Used to
    /// auto-expand only directories the user has never seen, so refreshes
    /// don't keep re-opening folders the user explicitly collapsed.
    known_file_dirs: HashSet<String>,
    pub tabs: tab_bar::TabState,
    pub changed_files: Vec<ChangedFile>,
    /// Per-change interaction states keyed by change name.
    /// Switching changes keeps previous sessions alive.
    pub interactions: HashMap<String, InteractionState>,
    /// Virtual exploration changes (not persisted to duckspec).
    pub explorations: Vec<String>,
    /// Counter for generating unique exploration names.
    pub exploration_counter: usize,
    /// Vertical scroll offset for the list column.
    pub list_scroll: f32,
}

impl State {
    pub fn new(project_root: Option<&Path>) -> Self {
        let mut sections = HashSet::new();
        sections.insert("picker".to_string());
        sections.insert("overview".to_string());
        sections.insert("capabilities".to_string());
        sections.insert("steps".to_string());
        sections.insert("changed_files".to_string());
        let (explorations, exploration_counter) =
            crate::chat_store::load_explorations(project_root);
        Self {
            selected_change: None,
            expanded_sections: sections,
            expanded_nodes: HashSet::new(),
            expanded_file_dirs: HashSet::new(),
            known_file_dirs: HashSet::new(),
            tabs: tab_bar::TabState::default(),
            changed_files: vec![],
            interactions: HashMap::new(),
            explorations,
            exploration_counter,
            list_scroll: 0.0,
        }
    }
}

impl State {
    /// Get the active change's interaction state (if a change is selected).
    pub fn active_interaction(&self) -> Option<&InteractionState> {
        let name = self.selected_change.as_ref()?;
        self.interactions.get(name)
    }

    /// Get the active change's interaction state mutably, creating it if needed.
    pub fn active_interaction_mut(&mut self) -> Option<&mut InteractionState> {
        let name = self.selected_change.as_ref()?;
        Some(self.interactions.entry(name.clone()).or_default())
    }

    /// Replace the changed-files list. Auto-expands only directories the
    /// user has never seen before, so a freshly-loaded changeset surfaces
    /// new files without re-opening folders the user explicitly collapsed
    /// during a previous refresh. Dirs that no longer appear are forgotten,
    /// so they auto-expand again if they ever come back.
    pub fn set_changed_files(&mut self, files: Vec<ChangedFile>) {
        let mut current_dirs: HashSet<String> = HashSet::new();
        for f in &files {
            let parts: Vec<&str> = f
                .path
                .components()
                .filter_map(|c| c.as_os_str().to_str())
                .collect();
            if parts.len() < 2 {
                continue;
            }
            let mut current = PathBuf::new();
            for part in &parts[..parts.len() - 1] {
                current.push(part);
                current_dirs.insert(current.display().to_string());
            }
        }

        for dir in &current_dirs {
            if !self.known_file_dirs.contains(dir) && !is_collapse_by_default(dir) {
                self.expanded_file_dirs.insert(dir.clone());
            }
        }
        self.expanded_file_dirs.retain(|d| current_dirs.contains(d));
        self.known_file_dirs = current_dirs;

        self.changed_files = files;
    }
}

/// Directories the changed-files tree should leave collapsed even on first
/// appearance. The duckspec root is usually noise — the user is typically
/// looking at the project's own changes, not their tracked spec edits — but
/// can still be expanded by hand when wanted.
fn is_collapse_by_default(dir: &str) -> bool {
    dir == "duckspec"
}

impl State {
    /// Whether the currently selected change is an exploration (virtual).
    pub fn is_exploration_selected(&self) -> bool {
        self.selected_change
            .as_ref()
            .is_some_and(|name| self.explorations.contains(name))
    }

    /// Promote an exploration to a real change: remove from explorations list,
    /// migrate interaction state and chat sessions to the new name.
    pub fn promote_exploration(
        &mut self,
        exploration_name: &str,
        real_name: &str,
        project_root: Option<&Path>,
    ) {
        self.explorations.retain(|n| n != exploration_name);
        if let Some(mut ix) = self.interactions.remove(exploration_name) {
            for ax in ix.sessions.iter_mut() {
                ax.session.scope = real_name.to_string();
            }
            interaction::reconcile_display_names(&mut ix.sessions);
            self.interactions.insert(real_name.to_string(), ix);
        }
        if self.selected_change.as_deref() == Some(exploration_name) {
            self.selected_change = Some(real_name.to_string());
        }
        crate::chat_store::rename_scope(exploration_name, real_name, project_root);
        crate::chat_store::save_explorations(
            &self.explorations,
            self.exploration_counter,
            project_root,
        );
    }

    /// Migrate interaction state and chat sessions from a change that was just
    /// archived externally (via CLI, agent tool, etc.) to its new archived name.
    /// Mirrors `promote_exploration` but without exploration bookkeeping.
    pub fn archive_change(
        &mut self,
        old_name: &str,
        archived_name: &str,
        project_root: Option<&Path>,
    ) {
        if let Some(mut ix) = self.interactions.remove(old_name) {
            for ax in ix.sessions.iter_mut() {
                ax.session.scope = archived_name.to_string();
            }
            interaction::reconcile_display_names(&mut ix.sessions);
            self.interactions.insert(archived_name.to_string(), ix);
        }
        if self.selected_change.as_deref() == Some(old_name) {
            self.selected_change = Some(archived_name.to_string());
        }
        rewrite_tab_ids_for_archive(&mut self.tabs, old_name, archived_name);
        crate::chat_store::rename_scope(old_name, archived_name, project_root);
    }
}

/// Rewrite tab IDs that reference a change being archived so breadcrumbs, the
/// path header below the tab bar, and content lookups point to the new archive
/// location. Handles artifact tabs (`changes/<old>/…`) and VCS diff tabs
/// (`vcs:…/changes/<old>/…`). Tab titles are unchanged (they're filenames).
fn rewrite_tab_ids_for_archive(tabs: &mut tab_bar::TabState, old_name: &str, archived_name: &str) {
    let artifact_old = format!("changes/{old_name}/");
    let artifact_new = format!("archive/{archived_name}/");
    let vcs_old = format!("/changes/{old_name}/");
    let vcs_new = format!("/archive/{archived_name}/");

    let rewrite = |id: &str| -> Option<String> {
        if let Some(rest) = id.strip_prefix(&artifact_old) {
            return Some(format!("{artifact_new}{rest}"));
        }
        if let Some(rest) = id.strip_prefix("vcs:")
            && let Some(idx) = rest.find(&vcs_old)
        {
            let (lead, tail) = rest.split_at(idx);
            let tail = &tail[vcs_old.len()..];
            return Some(format!("vcs:{lead}{vcs_new}{tail}"));
        }
        None
    };

    if let Some(tab) = tabs.preview.as_mut()
        && let Some(new_id) = rewrite(&tab.id)
    {
        tab.id = new_id;
    }
    for tab in tabs.file_tabs.iter_mut() {
        if let Some(new_id) = rewrite(&tab.id) {
            tab.id = new_id;
        }
    }
}

// ── Messages ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Message {
    SelectChange(String),
    ToggleSection(String),
    ToggleNode(String),
    SelectItem(String),
    SelectTab(usize),
    CloseTab(usize),
    Interaction(interaction::Msg),
    SelectChangedFile(PathBuf),
    ToggleFileDir(String),
    TabContent(tab_bar::TabContentMsg),
    AddExploration,
    RemoveExploration(String),
    /// Navigate to a change and open one of its artifacts.
    OpenArtifact {
        change: String,
        artifact_id: String,
    },
    ScrollList(f32),
}

// ── Update ───────────────────────────────────────────────────────────────────

pub fn update(
    state: &mut State,
    message: Message,
    project: &ProjectData,
    highlighter: &crate::highlight::SyntaxHighlighter,
) {
    match message {
        Message::SelectChange(name) => {
            state.selected_change = Some(name.clone());
            state.expanded_nodes.clear();

            // Expand tree nodes for real changes.
            if !state.explorations.contains(&name)
                && let Some(change) = project
                    .active_changes
                    .iter()
                    .chain(project.archived_changes.iter())
                    .find(|c| c.name == name)
            {
                crate::data::TreeNode::collect_parent_ids(
                    &change.cap_tree,
                    &mut state.expanded_nodes,
                );
            }

            // Auto-open interaction and ensure at least one session exists.
            let ix = state.interactions.entry(name.clone()).or_default();
            interaction::ensure_sessions(ix, &name, project.project_root.as_deref(), highlighter);
            if !ix.visible {
                ix.visible = true;
                if ix.mode == InteractionMode::Terminal {
                    interaction::spawn_terminal(ix);
                }
            }
        }
        Message::ToggleSection(id) => {
            if !state.expanded_sections.remove(&id) {
                state.expanded_sections.insert(id);
            }
        }
        Message::ToggleNode(id) => {
            if !state.expanded_nodes.remove(&id) {
                state.expanded_nodes.insert(id);
            }
        }
        Message::ToggleFileDir(id) => {
            if !state.expanded_file_dirs.remove(&id) {
                state.expanded_file_dirs.insert(id);
            }
        }
        Message::SelectItem(id) => {
            open_artifact(state, &id, project, highlighter);
        }
        Message::SelectTab(idx) => state.tabs.select(idx),
        Message::CloseTab(idx) => state.tabs.close(idx),
        Message::Interaction(msg) => {
            let scope = match state.selected_change.clone() {
                Some(n) => n,
                None => return,
            };
            match msg {
                interaction::Msg::NewSession => {
                    let Some(ix) = state.active_interaction_mut() else {
                        return;
                    };
                    interaction::ensure_sessions(
                        ix,
                        &scope,
                        project.project_root.as_deref(),
                        highlighter,
                    );
                    let new_session = AgentSession::new(scope.clone());
                    let _ = crate::chat_store::save_session(
                        &new_session.session,
                        project.project_root.as_deref(),
                    );
                    ix.sessions.insert(0, new_session);
                    ix.active_session = 0;
                    interaction::reconcile_display_names(&mut ix.sessions);
                }
                interaction::Msg::SelectSession(id) => {
                    let Some(ix) = state.active_interaction_mut() else {
                        return;
                    };
                    if let Some(idx) = ix.find_session_index(&id) {
                        ix.active_session = idx;
                    }
                }
                interaction::Msg::ClearSession => {
                    // Multi-session areas don't surface a Clear button, but
                    // handle it defensively by resetting the active session.
                    let Some(ix) = state.active_interaction_mut() else {
                        return;
                    };
                    clear_active_session(ix, &scope, project.project_root.as_deref());
                }
                other => {
                    let Some(ix) = state.active_interaction_mut() else {
                        return;
                    };
                    interaction::update_with_side_effects(
                        ix,
                        other,
                        &scope,
                        project.project_root.as_deref(),
                        highlighter,
                    );
                }
            }
        }
        Message::SelectChangedFile(path) => {
            if let Some(root) = &project.project_root
                && let Some(content) =
                    crate::widget::diff_view::build_diff_tab(root, &path, highlighter)
            {
                let id = format!("vcs:{}", path.display());
                let title = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.display().to_string());
                state
                    .tabs
                    .open_diff(id, title, content.editor, content.path, content.status);
            }
        }
        Message::TabContent(tab_bar::TabContentMsg::EditorAction(action)) => {
            crate::handle_editor_action(&mut state.tabs, action, highlighter);
        }
        Message::AddExploration => {
            state.exploration_counter += 1;
            let name = format!("Exploration {}", state.exploration_counter);
            state.explorations.push(name.clone());
            state.selected_change = Some(name.clone());
            crate::chat_store::save_explorations(
                &state.explorations,
                state.exploration_counter,
                project.project_root.as_deref(),
            );
            // Auto-open interaction panel with a fresh session.
            let ix = state.interactions.entry(name.clone()).or_default();
            interaction::ensure_sessions(ix, &name, project.project_root.as_deref(), highlighter);
            ix.visible = true;
            if ix.mode == InteractionMode::Terminal {
                interaction::spawn_terminal(ix);
            }
        }
        Message::RemoveExploration(name) => {
            state.explorations.retain(|n| n != &name);
            state.interactions.remove(&name);
            if state.selected_change.as_deref() == Some(&name) {
                state.selected_change = None;
            }
            crate::chat_store::delete_scope(&name, project.project_root.as_deref());
            crate::chat_store::save_explorations(
                &state.explorations,
                state.exploration_counter,
                project.project_root.as_deref(),
            );
        }
        Message::OpenArtifact {
            change,
            artifact_id,
        } => {
            state.selected_change = Some(change.clone());
            state.expanded_nodes.clear();
            if let Some(ch) = project
                .active_changes
                .iter()
                .chain(project.archived_changes.iter())
                .find(|c| c.name == change)
            {
                crate::data::TreeNode::collect_parent_ids(&ch.cap_tree, &mut state.expanded_nodes);
            }
            open_artifact(state, &artifact_id, project, highlighter);
        }
        Message::ScrollList(offset) => {
            state.list_scroll = offset;
        }
    }

    refresh_obvious_command(state, project);
}

/// Compute the suggested next /ds-* command (without the leading slash) given
/// the selected change's artifact state. Returns `None` for archived changes
/// or when nothing is selected.
pub fn compute_obvious_command(state: &State, project: &ProjectData) -> Option<String> {
    let selected = state.selected_change.as_deref()?;

    // Exploration (virtual) — always orient first.
    if state.explorations.iter().any(|e| e == selected) {
        return Some("ds-explore".into());
    }

    // Archived changes are terminal — no further action.
    if project.archived_changes.iter().any(|c| c.name == selected) {
        return None;
    }

    let change = project.active_changes.iter().find(|c| c.name == selected)?;

    // Steps exist → either apply (unfinished) or archive (all done).
    if !change.steps.is_empty() {
        let all_done = change
            .steps
            .iter()
            .all(|s| matches!(s.completion, StepCompletion::Done));
        return Some(if all_done {
            "ds-archive".into()
        } else {
            "ds-apply".into()
        });
    }

    // Caps exist → feature flow needs steps next; refinement/doc-only is ready to archive.
    if !change.cap_tree.is_empty() {
        return Some(if change.has_design {
            "ds-step".into()
        } else {
            "ds-archive".into()
        });
    }

    // No caps yet — walk the feature-flow ladder.
    if change.has_design {
        return Some("ds-spec".into());
    }
    if change.has_proposal {
        return Some("ds-design".into());
    }
    Some("ds-propose".into())
}

/// Refresh the `obvious_command` on every session of the active interaction.
/// Call after update() and after project reload.
pub fn refresh_obvious_command(state: &mut State, project: &ProjectData) {
    let cmd = compute_obvious_command(state, project);
    let Some(name) = state.selected_change.clone() else {
        return;
    };
    let Some(ix) = state.interactions.get_mut(&name) else {
        return;
    };
    for ax in ix.sessions.iter_mut() {
        ax.obvious_command = cmd.clone();
    }
}

// ── View ─────────────────────────────────────────────────────────────────────

pub fn view<'a>(state: &'a State, project: &'a ProjectData) -> Element<'a, Message> {
    let list = view_list(state, project);
    let divider = container(Space::new().height(Length::Fill))
        .width(1.0)
        .style(theme::divider);

    let is_exploration = state.is_exploration_selected();
    let ix = state.active_interaction();

    // Exploration mode: no content column until a file is opened. Once tabs
    // exist, fall back to the normal list | content | toggle | interaction
    // layout so the opened file is actually visible.
    if is_exploration {
        let has_tabs = state.tabs.preview.is_some() || !state.tabs.file_tabs.is_empty();

        let mut main_row = row![
            container(list)
                .width(theme::LIST_COLUMN_WIDTH)
                .height(Length::Fill)
                .style(theme::surface),
            divider,
        ];

        if has_tabs {
            let content = view_content(state, project);
            main_row = main_row.push(container(content).width(Length::Fill).height(Length::Fill));

            let visible = ix.is_some_and(|i| i.visible);
            let width = ix.map_or(theme::INTERACTION_COLUMN_WIDTH, |i| i.width);
            let toggle = interaction_toggle::view(visible, width, |m| {
                Message::Interaction(interaction::Msg::Handle(m))
            });
            main_row = main_row.push(toggle);

            if let Some(ix) = ix
                && ix.visible
            {
                let interaction_col =
                    interaction::view_column(ix, Message::Interaction, SessionControls::Single);
                main_row = main_row.push(
                    container(interaction_col)
                        .width(ix.width)
                        .height(Length::Fill)
                        .style(theme::surface),
                );
            }
        } else if let Some(ix) = ix {
            let interaction_col =
                interaction::view_column(ix, Message::Interaction, SessionControls::Single);
            main_row = main_row.push(
                container(interaction_col)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(theme::surface),
            );
        }

        return main_row.height(Length::Fill).into();
    }

    // Normal mode: content column + optional interaction panel.
    let content = view_content(state, project);
    let visible = ix.is_some_and(|i| i.visible);
    let width = ix.map_or(theme::INTERACTION_COLUMN_WIDTH, |i| i.width);

    let toggle = interaction_toggle::view(visible, width, |m| {
        Message::Interaction(interaction::Msg::Handle(m))
    });

    let mut main_row = row![
        container(list)
            .width(theme::LIST_COLUMN_WIDTH)
            .height(Length::Fill)
            .style(theme::surface),
        divider,
        container(content).width(Length::Fill).height(Length::Fill),
        toggle,
    ];

    if let Some(ix) = ix
        && ix.visible
    {
        let interaction_col =
            interaction::view_column(ix, Message::Interaction, SessionControls::Multi);

        main_row = main_row.push(
            container(interaction_col)
                .width(ix.width)
                .height(Length::Fill)
                .style(theme::surface),
        );
    }

    main_row.height(Length::Fill).into()
}

// ── Breadcrumbs ──────────────────────────────────────────────────────────────

pub fn breadcrumbs(state: &State, project: &ProjectData) -> Vec<String> {
    let Some(selected) = state.selected_change.as_deref() else {
        return vec!["Changes".into()];
    };

    // Exploration mode renders no tab; show only the exploration root.
    if state.explorations.iter().any(|e| e == selected) {
        return vec!["Explorations".into(), selected.into()];
    }

    let is_archived = project.archived_changes.iter().any(|c| c.name == selected);

    if let Some(tab) = state.tabs.active_tab() {
        return tab_breadcrumbs(&tab.id, selected, is_archived);
    }

    let root = if is_archived { "Archive" } else { "Changes" };
    vec![root.into(), selected.into()]
}

fn tab_breadcrumbs(id: &str, selected: &str, selected_archived: bool) -> Vec<String> {
    if let Some(path) = id.strip_prefix("file:") {
        return vec!["Files".into(), path.into()];
    }

    if let Some(path) = id.strip_prefix("vcs:") {
        let root = if selected_archived {
            "Archive"
        } else {
            "Changes"
        };
        return vec![
            root.into(),
            selected.into(),
            "Changed files".into(),
            path.into(),
        ];
    }

    let root_rest = id
        .strip_prefix("changes/")
        .map(|r| ("Changes", r))
        .or_else(|| id.strip_prefix("archive/").map(|r| ("Archive", r)));

    if let Some((root, rest)) = root_rest {
        let (change, inner) = rest.split_once('/').unwrap_or((rest, ""));
        let mut segs = vec![root.into(), change.into()];
        segs.extend(parse_change_inner(inner));
        return segs;
    }

    vec![id.into()]
}

fn parse_change_inner(path: &str) -> Vec<String> {
    if path.is_empty() {
        return vec![];
    }
    if path == "proposal.md" {
        return vec!["Proposal".into()];
    }
    if path == "design.md" {
        return vec!["Design".into()];
    }
    if let Some(rest) = path.strip_prefix("caps/") {
        let mut segs = vec!["Capabilities".into()];
        segs.extend(rest.split('/').map(str::to_string));
        return segs;
    }
    if let Some(rest) = path.strip_prefix("steps/") {
        return vec!["Steps".into(), rest.into()];
    }
    path.split('/').map(str::to_string).collect()
}

#[cfg(test)]
mod breadcrumb_tests {
    use super::*;

    #[test]
    fn tab_proposal() {
        assert_eq!(
            tab_breadcrumbs("changes/foo/proposal.md", "foo", false),
            vec!["Changes", "foo", "Proposal"]
        );
    }

    #[test]
    fn tab_design() {
        assert_eq!(
            tab_breadcrumbs("changes/foo/design.md", "foo", false),
            vec!["Changes", "foo", "Design"]
        );
    }

    #[test]
    fn tab_step() {
        assert_eq!(
            tab_breadcrumbs("changes/foo/steps/01-bar.md", "foo", false),
            vec!["Changes", "foo", "Steps", "01-bar.md"]
        );
    }

    #[test]
    fn tab_cap_nested() {
        assert_eq!(
            tab_breadcrumbs("changes/foo/caps/auth/session.md", "foo", false),
            vec!["Changes", "foo", "Capabilities", "auth", "session.md"]
        );
    }

    #[test]
    fn tab_cap_deeply_nested() {
        assert_eq!(
            tab_breadcrumbs("changes/foo/caps/a/b/c/d.md", "foo", false),
            vec!["Changes", "foo", "Capabilities", "a", "b", "c", "d.md"]
        );
    }

    #[test]
    fn tab_archive_proposal() {
        assert_eq!(
            tab_breadcrumbs(
                "archive/2026-04-20-01-foo/proposal.md",
                "2026-04-20-01-foo",
                true
            ),
            vec!["Archive", "2026-04-20-01-foo", "Proposal"]
        );
    }

    #[test]
    fn tab_vcs_active() {
        assert_eq!(
            tab_breadcrumbs("vcs:src/main.rs", "foo", false),
            vec!["Changes", "foo", "Changed files", "src/main.rs"]
        );
    }

    #[test]
    fn tab_vcs_archived() {
        assert_eq!(
            tab_breadcrumbs("vcs:src/main.rs", "2026-04-20-01-foo", true),
            vec![
                "Archive",
                "2026-04-20-01-foo",
                "Changed files",
                "src/main.rs"
            ]
        );
    }

    #[test]
    fn tab_file_finder() {
        assert_eq!(
            tab_breadcrumbs("file:Cargo.toml", "foo", false),
            vec!["Files", "Cargo.toml"]
        );
    }

    #[test]
    fn tab_unknown_falls_back() {
        assert_eq!(tab_breadcrumbs("weird-id", "foo", false), vec!["weird-id"]);
    }

    fn make_state(selected: &str, explorations: &[&str]) -> State {
        State {
            selected_change: Some(selected.to_string()),
            expanded_sections: HashSet::new(),
            expanded_nodes: HashSet::new(),
            expanded_file_dirs: HashSet::new(),
            tabs: tab_bar::TabState::default(),
            changed_files: vec![],
            interactions: HashMap::new(),
            explorations: explorations.iter().map(|s| s.to_string()).collect(),
            exploration_counter: 0,
            list_scroll: 0.0,
            known_file_dirs: HashSet::new(),
        }
    }

    fn make_project(active: &[&str], archived: &[&str]) -> ProjectData {
        use crate::data::ChangeData;
        let mk = |name: &str, prefix_root: &str| ChangeData {
            name: name.to_string(),
            prefix: format!("{prefix_root}/{name}"),
            has_proposal: false,
            has_design: false,
            cap_tree: vec![],
            steps: vec![],
        };
        ProjectData {
            active_changes: active.iter().map(|n| mk(n, "changes")).collect(),
            archived_changes: archived.iter().map(|n| mk(n, "archive")).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn exploration_root_after_selection() {
        let state = make_state("Exploration 1", &["Exploration 1"]);
        let project = make_project(&[], &[]);
        assert_eq!(
            breadcrumbs(&state, &project),
            vec!["Explorations", "Exploration 1"]
        );
    }

    #[test]
    fn exploration_promoted_to_change_shows_changes_root() {
        // After promote_exploration: selected_change → new name,
        // explorations list no longer contains the old name,
        // active_changes contains the new name.
        let state = make_state("real-change", &[]);
        let project = make_project(&["real-change"], &[]);
        assert_eq!(
            breadcrumbs(&state, &project),
            vec!["Changes", "real-change"]
        );
    }

    #[test]
    fn change_archived_shows_archive_root() {
        // After archive_change: selected_change → archived name,
        // archived_changes contains it.
        let state = make_state("2026-04-20-01-foo", &[]);
        let project = make_project(&[], &["2026-04-20-01-foo"]);
        assert_eq!(
            breadcrumbs(&state, &project),
            vec!["Archive", "2026-04-20-01-foo"]
        );
    }

    // ── compute_obvious_command ─────────────────────────────────────────────

    fn tree_node(id: &str) -> crate::data::TreeNode {
        crate::data::TreeNode {
            id: id.into(),
            label: id.into(),
            children: vec![],
        }
    }

    fn step(done: bool) -> crate::data::StepInfo {
        crate::data::StepInfo {
            id: "changes/foo/steps/01-bar.md".into(),
            label: "01-bar.md".into(),
            completion: if done {
                StepCompletion::Done
            } else {
                StepCompletion::Partial(0, 1)
            },
        }
    }

    fn set_change(project: &mut ProjectData, name: &str, mutate: impl FnOnce(&mut ChangeData)) {
        let ch = project
            .active_changes
            .iter_mut()
            .find(|c| c.name == name)
            .expect("change exists");
        mutate(ch);
    }

    #[test]
    fn obvious_nothing_selected() {
        let state = State {
            selected_change: None,
            expanded_sections: HashSet::new(),
            expanded_nodes: HashSet::new(),
            expanded_file_dirs: HashSet::new(),
            tabs: tab_bar::TabState::default(),
            changed_files: vec![],
            interactions: HashMap::new(),
            explorations: vec![],
            exploration_counter: 0,
            list_scroll: 0.0,
            known_file_dirs: HashSet::new(),
        };
        let project = make_project(&[], &[]);
        assert_eq!(compute_obvious_command(&state, &project), None);
    }

    #[test]
    fn obvious_exploration_always_explore() {
        let state = make_state("Exploration 1", &["Exploration 1"]);
        let project = make_project(&[], &[]);
        assert_eq!(
            compute_obvious_command(&state, &project).as_deref(),
            Some("ds-explore")
        );
    }

    #[test]
    fn obvious_archived_is_none() {
        let state = make_state("2026-04-20-01-foo", &[]);
        let project = make_project(&[], &["2026-04-20-01-foo"]);
        assert_eq!(compute_obvious_command(&state, &project), None);
    }

    #[test]
    fn obvious_empty_change_suggests_propose() {
        let state = make_state("foo", &[]);
        let project = make_project(&["foo"], &[]);
        assert_eq!(
            compute_obvious_command(&state, &project).as_deref(),
            Some("ds-propose")
        );
    }

    #[test]
    fn obvious_with_proposal_suggests_design() {
        let state = make_state("foo", &[]);
        let mut project = make_project(&["foo"], &[]);
        set_change(&mut project, "foo", |c| c.has_proposal = true);
        assert_eq!(
            compute_obvious_command(&state, &project).as_deref(),
            Some("ds-design")
        );
    }

    #[test]
    fn obvious_with_design_suggests_spec() {
        let state = make_state("foo", &[]);
        let mut project = make_project(&["foo"], &[]);
        set_change(&mut project, "foo", |c| {
            c.has_proposal = true;
            c.has_design = true;
        });
        assert_eq!(
            compute_obvious_command(&state, &project).as_deref(),
            Some("ds-spec")
        );
    }

    #[test]
    fn obvious_feature_flow_with_caps_suggests_step() {
        let state = make_state("foo", &[]);
        let mut project = make_project(&["foo"], &[]);
        set_change(&mut project, "foo", |c| {
            c.has_proposal = true;
            c.has_design = true;
            c.cap_tree = vec![tree_node("caps/auth")];
        });
        assert_eq!(
            compute_obvious_command(&state, &project).as_deref(),
            Some("ds-step")
        );
    }

    #[test]
    fn obvious_refinement_with_caps_suggests_archive() {
        // Spec refinement / doc-only: caps present but no design.
        let state = make_state("foo", &[]);
        let mut project = make_project(&["foo"], &[]);
        set_change(&mut project, "foo", |c| {
            c.cap_tree = vec![tree_node("caps/auth")];
        });
        assert_eq!(
            compute_obvious_command(&state, &project).as_deref(),
            Some("ds-archive")
        );
    }

    #[test]
    fn obvious_steps_unfinished_suggests_apply() {
        let state = make_state("foo", &[]);
        let mut project = make_project(&["foo"], &[]);
        set_change(&mut project, "foo", |c| {
            c.has_proposal = true;
            c.has_design = true;
            c.cap_tree = vec![tree_node("caps/auth")];
            c.steps = vec![step(false), step(true)];
        });
        assert_eq!(
            compute_obvious_command(&state, &project).as_deref(),
            Some("ds-apply")
        );
    }

    // ── rewrite_tab_ids_for_archive ─────────────────────────────────────────

    fn make_tab(id: &str) -> crate::widget::tab_bar::Tab {
        crate::widget::tab_bar::Tab {
            id: id.into(),
            title: id.rsplit('/').next().unwrap_or(id).into(),
            view: crate::widget::tab_bar::TabView::Editor {
                editor: crate::widget::text_edit::EditorState::new(""),
            },
        }
    }

    #[test]
    fn rewrite_rewrites_artifact_preview_and_file_tabs() {
        let mut tabs = tab_bar::TabState {
            preview: Some(make_tab("changes/foo/proposal.md")),
            file_tabs: vec![
                make_tab("changes/foo/caps/auth/spec.md"),
                make_tab("changes/bar/proposal.md"),
                make_tab("file:Cargo.toml"),
            ],
            active: Default::default(),
        };

        rewrite_tab_ids_for_archive(&mut tabs, "foo", "2026-04-20-01-foo");

        assert_eq!(
            tabs.preview.as_ref().map(|t| t.id.as_str()),
            Some("archive/2026-04-20-01-foo/proposal.md"),
        );
        assert_eq!(
            tabs.file_tabs[0].id,
            "archive/2026-04-20-01-foo/caps/auth/spec.md"
        );
        // Unrelated change left alone.
        assert_eq!(tabs.file_tabs[1].id, "changes/bar/proposal.md");
        // Non-change tab left alone.
        assert_eq!(tabs.file_tabs[2].id, "file:Cargo.toml");
    }

    #[test]
    fn rewrite_rewrites_vcs_tab_ids() {
        let mut tabs = tab_bar::TabState {
            preview: Some(make_tab("vcs:duckspec/changes/foo/proposal.md")),
            file_tabs: vec![],
            active: Default::default(),
        };

        rewrite_tab_ids_for_archive(&mut tabs, "foo", "2026-04-20-01-foo");

        assert_eq!(
            tabs.preview.as_ref().map(|t| t.id.as_str()),
            Some("vcs:duckspec/archive/2026-04-20-01-foo/proposal.md"),
        );
    }

    #[test]
    fn rewrite_leaves_similar_but_different_names_alone() {
        // "foo2" must not match "foo".
        let mut tabs = tab_bar::TabState {
            preview: Some(make_tab("changes/foo2/proposal.md")),
            file_tabs: vec![],
            active: Default::default(),
        };

        rewrite_tab_ids_for_archive(&mut tabs, "foo", "2026-04-20-01-foo");

        assert_eq!(
            tabs.preview.as_ref().map(|t| t.id.as_str()),
            Some("changes/foo2/proposal.md"),
        );
    }

    #[test]
    fn obvious_all_steps_done_suggests_archive() {
        let state = make_state("foo", &[]);
        let mut project = make_project(&["foo"], &[]);
        set_change(&mut project, "foo", |c| {
            c.has_proposal = true;
            c.has_design = true;
            c.cap_tree = vec![tree_node("caps/auth")];
            c.steps = vec![step(true), step(true)];
        });
        assert_eq!(
            compute_obvious_command(&state, &project).as_deref(),
            Some("ds-archive")
        );
    }
}

/// Reset the active session for a scope: cancel agent, delete persisted file,
/// and replace with a fresh empty session under a new id.
fn clear_active_session(ix: &mut InteractionState, scope: &str, project_root: Option<&Path>) {
    if ix.sessions.is_empty() {
        ix.sessions.push(AgentSession::new(scope.to_string()));
        ix.active_session = 0;
        return;
    }
    let idx = ix.active_session.min(ix.sessions.len() - 1);
    if let Some(ax) = ix.sessions.get(idx) {
        if let Some(handle) = &ax.agent_handle {
            handle.cancel();
        }
        crate::chat_store::delete_session(&ax.session.scope, &ax.session.id, project_root);
    }
    ix.sessions[idx] = AgentSession::new(scope.to_string());
    ix.active_session = idx;
    interaction::reconcile_display_names(&mut ix.sessions);
}

fn view_list<'a>(state: &'a State, project: &'a ProjectData) -> Element<'a, Message> {
    let mut rows: Vec<ListRow<'a, Message>> = vec![];

    // Exploration changes (virtual) — listed first.
    for name in &state.explorations {
        let is_selected = state.selected_change.as_deref() == Some(name.as_str());
        let close_btn = collapsible::close_button(Message::RemoveExploration(name.clone()));
        rows.push(
            ListRow::new(name.as_str())
                .icon(ICON_EXPLORE)
                .sticky_trailing(close_btn)
                .selected(is_selected)
                .on_press(Message::SelectChange(name.clone())),
        );
    }

    // Active changes from duckspec.
    for ch in &project.active_changes {
        let is_selected = state.selected_change.as_ref() == Some(&ch.name);
        let mut r = ListRow::new(ch.name.as_str())
            .icon(ICON_BRANCH)
            .selected(is_selected)
            .on_press(Message::SelectChange(ch.name.clone()));
        if let Some(v) = project.validations.get(&ch.name) {
            let count = v.total_count();
            if count > 0 {
                r = r.badge(Badge::ErrorCount(count as u32));
            }
        }
        rows.push(r);
    }

    let selector = list_view::view(rows, None);

    // Archived changes — separate collapsible section.
    let archived_rows: Vec<ListRow<'a, Message>> = project
        .archived_changes
        .iter()
        .map(|ch| {
            let is_selected = state.selected_change.as_ref() == Some(&ch.name);
            let mut r = ListRow::new(ch.name.as_str())
                .icon(ICON_BRANCH)
                .selected(is_selected)
                .on_press(Message::SelectChange(ch.name.clone()));
            if let Some(v) = project.validations.get(&ch.name) {
                let count = v.total_count();
                if count > 0 {
                    r = r.badge(Badge::ErrorCount(count as u32));
                }
            }
            r
        })
        .collect();

    let archived_section = if project.archived_changes.is_empty() {
        None
    } else {
        Some(collapsible::view(
            "Archived",
            state.expanded_sections.contains("archived"),
            Message::ToggleSection("archived".to_string()),
            list_view::view(archived_rows, None),
        ))
    };

    let change_section = {
        let expanded = state.expanded_sections.contains("picker");
        let header = button(
            row![
                collapsible::chevron(expanded),
                text("CHANGE")
                    .size(theme::font_sm())
                    .color(theme::text_secondary()),
            ]
            .spacing(theme::SPACING_XS)
            .align_y(iced::Center)
            .width(Length::Fill),
        )
        .on_press(Message::ToggleSection("picker".to_string()))
        .width(Length::Fill)
        .style(theme::section_header)
        .padding([theme::SPACING_XS, theme::SPACING_SM]);

        let mut col = column![row![
            container(header).width(Length::Fill),
            collapsible::add_button(Message::AddExploration),
        ]]
        .spacing(0.0);

        if expanded {
            col = col.push(collapsible::top_divider());
            col = col.push(selector);
        }
        col
    };

    let change = find_change(state, project);
    let is_exploration = state.is_exploration_selected();
    let mut list_col = column![change_section].spacing(0.0);

    if let Some(section) = archived_section {
        list_col = list_col.push(section);
    }

    if is_exploration {
        // Explorations only show the interaction column, no overview/caps/steps.
        list_col = list_col.push(
            container(
                text("Exploration mode — use the agent or terminal to work freely.")
                    .size(theme::font_md())
                    .color(theme::text_muted()),
            )
            .padding([theme::SPACING_SM, theme::SPACING_SM]),
        );
    } else if let Some(change) = change {
        let error_ids: HashSet<String> = project
            .validations
            .get(&change.name)
            .map(|v| v.file_errors.iter().map(|(p, _)| p.clone()).collect())
            .unwrap_or_default();
        list_col = list_col.push(view_overview_section(state, change, &error_ids));
        list_col = list_col.push(view_caps_section(state, change, &error_ids));
        list_col = list_col.push(view_steps_section(state, change, &error_ids));
    }

    // Changed files section (always visible, independent of selected change).
    list_col = list_col.push(view_changed_files_section(state));

    vertical_scroll::view(state.list_scroll, Message::ScrollList, list_col)
}

fn view_overview_section<'a>(
    state: &'a State,
    change: &'a ChangeData,
    error_ids: &HashSet<String>,
) -> Element<'a, Message> {
    let active_id = state.tabs.active_tab().map(|t| t.id.as_str());
    let mut rows: Vec<ListRow<'a, Message>> = vec![];

    let mut push_file = |label: &'static str, id: String, has_err: bool| {
        let mut r = ListRow::new(label)
            .icon(icon_for_artifact(label))
            .selected(active_id == Some(id.as_str()))
            .on_press(Message::SelectItem(id));
        if has_err {
            r = r.badge(Badge::ErrorDot);
        }
        rows.push(r);
    };

    if change.has_proposal {
        let id = format!("{}/proposal.md", change.prefix);
        let has_err = error_ids.contains(&id);
        push_file("proposal.md", id, has_err);
    }
    if change.has_design {
        let id = format!("{}/design.md", change.prefix);
        let has_err = error_ids.contains(&id);
        push_file("design.md", id, has_err);
    }

    collapsible::view(
        "Overview",
        state.expanded_sections.contains("overview"),
        Message::ToggleSection("overview".to_string()),
        list_view::view(rows, Some("No overview files")),
    )
}

fn view_caps_section<'a>(
    state: &'a State,
    change: &'a ChangeData,
    error_ids: &HashSet<String>,
) -> Element<'a, Message> {
    let content = if change.cap_tree.is_empty() {
        container(
            text("No capability changes")
                .size(theme::font_md())
                .color(theme::text_muted()),
        )
        .padding([theme::SPACING_XS, theme::SPACING_SM])
        .into()
    } else {
        tree_view::view(
            &change.cap_tree,
            &state.expanded_nodes,
            state.tabs.active_tab().map(|t| t.id.as_str()),
            error_ids,
            Message::ToggleNode,
            Message::SelectItem,
        )
    };

    collapsible::view(
        "Capabilities",
        state.expanded_sections.contains("capabilities"),
        Message::ToggleSection("capabilities".to_string()),
        content,
    )
}

fn view_steps_section<'a>(
    state: &'a State,
    change: &'a ChangeData,
    error_ids: &HashSet<String>,
) -> Element<'a, Message> {
    let active_id = state.tabs.active_tab().map(|t| t.id.as_str());
    let rows: Vec<ListRow<'a, Message>> = change
        .steps
        .iter()
        .map(|step| {
            let (icon_bytes, icon_tint): (&'static [u8], Option<iced::Color>) =
                match step.completion {
                    StepCompletion::Done => (ICON_STEP_DONE, Some(theme::success())),
                    StepCompletion::Partial(0, _) | StepCompletion::NoTasks => (ICON_STEP, None),
                    StepCompletion::Partial(_, _) => (ICON_STEP_PARTIAL, Some(theme::warning())),
                };
            let mut r = ListRow::new(step.label.as_str())
                .icon(icon_bytes)
                .selected(active_id == Some(step.id.as_str()))
                .on_press(Message::SelectItem(step.id.clone()));
            if let Some(tint) = icon_tint {
                r = r.icon_tint(tint);
            }
            if error_ids.contains(&step.id) {
                r = r.badge(Badge::ErrorDot);
            }
            r
        })
        .collect();

    collapsible::view(
        "Steps",
        state.expanded_sections.contains("steps"),
        Message::ToggleSection("steps".to_string()),
        list_view::view(rows, Some("No steps")),
    )
}

/// Tree of changed files grouped by directory.
///
/// Built from the flat list of `ChangedFile`s so that whole-directory additions
/// can be rendered as collapsible nodes rather than one opaque entry.
struct FileTree {
    /// Child directories, keyed by directory name (BTreeMap for sorted order).
    dirs: BTreeMap<String, FileTree>,
    /// Files directly inside this directory (full repo-relative path retained).
    files: Vec<ChangedFile>,
    /// Repo-relative path of this directory. Empty for the root.
    path: PathBuf,
}

impl FileTree {
    fn new(path: PathBuf) -> Self {
        Self {
            dirs: BTreeMap::new(),
            files: vec![],
            path,
        }
    }

    fn insert(&mut self, file: ChangedFile) {
        let parts: Vec<String> = file
            .path
            .components()
            .filter_map(|c| c.as_os_str().to_str().map(|s| s.to_string()))
            .collect();
        if parts.is_empty() {
            return;
        }
        let mut node = self;
        let mut current_path = PathBuf::new();
        for part in &parts[..parts.len() - 1] {
            current_path.push(part);
            node = node
                .dirs
                .entry(part.clone())
                .or_insert_with(|| FileTree::new(current_path.clone()));
        }
        node.files.push(file);
    }
}

/// Aggregate status for a directory: `Some(status)` if every descendant file
/// shares the same status, `None` if mixed or empty.
fn aggregate_status(node: &FileTree) -> Option<FileStatus> {
    fn visit(node: &FileTree, seen: &mut Option<FileStatus>) -> bool {
        for file in &node.files {
            match seen {
                None => *seen = Some(file.status),
                Some(s) if *s == file.status => {}
                Some(_) => return false,
            }
        }
        for sub in node.dirs.values() {
            if !visit(sub, seen) {
                return false;
            }
        }
        true
    }
    let mut seen = None;
    if visit(node, &mut seen) { seen } else { None }
}

enum FileTreeRow<'a> {
    Dir {
        key: String,
        name: String,
        depth: usize,
        is_expanded: bool,
        agg: Option<FileStatus>,
    },
    File {
        file: &'a ChangedFile,
        depth: usize,
    },
}

fn flatten_file_tree<'a>(
    node: &'a FileTree,
    depth: usize,
    expanded: &HashSet<String>,
    out: &mut Vec<FileTreeRow<'a>>,
) {
    for (name, sub) in &node.dirs {
        let key = sub.path.display().to_string();
        let is_expanded = expanded.contains(&key);
        let agg = aggregate_status(sub);
        out.push(FileTreeRow::Dir {
            key,
            name: name.clone(),
            depth,
            is_expanded,
            agg,
        });
        if is_expanded {
            flatten_file_tree(sub, depth + 1, expanded, out);
        }
    }
    let mut files: Vec<&ChangedFile> = node.files.iter().collect();
    files.sort_by_key(|f| {
        f.path
            .file_name()
            .map(|s| s.to_os_string())
            .unwrap_or_default()
    });
    for file in files {
        out.push(FileTreeRow::File { file, depth });
    }
}

fn status_char(status: FileStatus) -> &'static str {
    match status {
        FileStatus::Modified => "M",
        FileStatus::Added => "A",
        FileStatus::Deleted => "D",
    }
}

fn view_changed_files_section<'a>(state: &'a State) -> Element<'a, Message> {
    let rows: Vec<ListRow<'a, Message>> = if state.changed_files.is_empty() {
        vec![]
    } else {
        let mut tree = FileTree::new(PathBuf::new());
        for cf in &state.changed_files {
            tree.insert(cf.clone());
        }
        let mut flat = Vec::new();
        flatten_file_tree(&tree, 0, &state.expanded_file_dirs, &mut flat);

        let active_tab_id = state.tabs.active_tab().map(|t| t.id.as_str());

        flat.into_iter()
            .map(|row_data| match row_data {
                FileTreeRow::Dir {
                    key,
                    name,
                    depth,
                    is_expanded,
                    agg,
                } => {
                    let (sc, color) = match agg {
                        Some(s) => (status_char(s), theme::vcs_status_color(&s)),
                        None => ("~", theme::text_muted()),
                    };
                    let leading: Element<'a, Message> = row![
                        collapsible::chevron(is_expanded),
                        text(sc)
                            .size(theme::font_md())
                            .font(theme::content_font())
                            .color(color),
                    ]
                    .spacing(theme::SPACING_SM)
                    .align_y(iced::Center)
                    .into();
                    ListRow::new(format!("{}/", name))
                        .leading(leading)
                        .indent(depth)
                        .spacing(theme::SPACING_SM)
                        .on_press(Message::ToggleFileDir(key))
                }
                FileTreeRow::File { file, depth } => {
                    let sc = status_char(file.status);
                    let color = theme::vcs_status_color(&file.status);
                    let tab_id = format!("vcs:{}", file.path.display());
                    let is_active = active_tab_id == Some(tab_id.as_str());
                    let name = file
                        .path
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| file.path.display().to_string());
                    let leading: Element<'a, Message> = row![
                        // Keep file rows aligned with the arrow column used by dirs.
                        Space::new().width(theme::font_sm()),
                        text(sc)
                            .size(theme::font_md())
                            .font(theme::content_font())
                            .color(color),
                    ]
                    .spacing(theme::SPACING_SM)
                    .align_y(iced::Center)
                    .into();
                    ListRow::new(name)
                        .leading(leading)
                        .indent(depth)
                        .spacing(theme::SPACING_SM)
                        .selected(is_active)
                        .on_press(Message::SelectChangedFile(file.path.clone()))
                }
            })
            .collect()
    };

    collapsible::view(
        "Changed Files",
        state.expanded_sections.contains("changed_files"),
        Message::ToggleSection("changed_files".to_string()),
        list_view::view(rows, Some("No changes")),
    )
}

fn icon_for_artifact(label: &str) -> &'static [u8] {
    match label {
        l if l.starts_with("spec.delta") => ICON_SPEC_DELTA,
        l if l.starts_with("spec") => ICON_SPEC,
        l if l.starts_with("doc.delta") => ICON_DOC_DELTA,
        l if l.starts_with("doc") => ICON_DOC,
        _ => ICON_FILE,
    }
}

fn view_content<'a>(state: &'a State, project: &'a ProjectData) -> Element<'a, Message> {
    let bar = tab_bar::view_bar(&state.tabs, Message::SelectTab, Message::CloseTab);
    let body = tab_bar::view_content(&state.tabs).map(Message::TabContent);

    // Error panel for the active artifact.
    let error_panel = state
        .tabs
        .active_tab()
        .and_then(|tab| {
            let change_name = state.selected_change.as_ref()?;
            let validation = project.validations.get(change_name)?;
            let errors = validation
                .file_errors
                .iter()
                .find(|(path, _)| *path == tab.id)?;
            Some(&errors.1)
        })
        .filter(|errs| !errs.is_empty());

    let mut col = column![bar, body].height(Length::Fill);

    if let Some(errors) = error_panel {
        let divider = container(Space::new().width(Length::Fill))
            .height(1.0)
            .style(theme::divider);

        let mut error_list = column![].spacing(theme::SPACING_XS);
        for err in errors {
            error_list = error_list.push(
                text(err.as_str())
                    .size(theme::font_md())
                    .color(theme::error()),
            );
        }

        let panel = container(
            column![
                text("Errors")
                    .size(theme::font_sm())
                    .color(theme::text_secondary()),
                error_list,
            ]
            .spacing(theme::SPACING_SM),
        )
        .padding(theme::SPACING_SM)
        .width(Length::Fill)
        .style(theme::surface);

        col = col.push(divider);
        col = col.push(panel);
    }

    col.into()
}

fn find_change<'a>(state: &State, project: &'a ProjectData) -> Option<&'a ChangeData> {
    let name = state.selected_change.as_ref()?;
    project
        .active_changes
        .iter()
        .chain(project.archived_changes.iter())
        .find(|c| &c.name == name)
}

fn open_artifact(
    state: &mut State,
    id: &str,
    project: &ProjectData,
    highlighter: &crate::highlight::SyntaxHighlighter,
) {
    if let Some(content) = project.read_artifact(id) {
        let title = id.rsplit('/').next().unwrap_or(id).to_string();
        crate::open_artifact_tab(
            &mut state.tabs,
            id.to_string(),
            title,
            content,
            id,
            highlighter,
        );
    }
}

#[cfg(test)]
mod file_tree_tests {
    use super::*;

    fn cf(path: &str, status: FileStatus) -> ChangedFile {
        ChangedFile {
            path: PathBuf::from(path),
            status,
        }
    }

    #[test]
    fn root_file_lands_at_depth_zero() {
        let mut t = FileTree::new(PathBuf::new());
        t.insert(cf("main.rs", FileStatus::Modified));
        assert!(t.dirs.is_empty());
        assert_eq!(t.files.len(), 1);
    }

    #[test]
    fn nested_paths_create_directories() {
        let mut t = FileTree::new(PathBuf::new());
        t.insert(cf(".claude/foo.md", FileStatus::Added));
        t.insert(cf(".claude/bar/baz.md", FileStatus::Added));
        t.insert(cf("agents/x.md", FileStatus::Added));

        assert_eq!(t.dirs.len(), 2);
        let claude = t.dirs.get(".claude").expect("dir");
        assert_eq!(claude.files.len(), 1);
        assert_eq!(claude.dirs.len(), 1);
        assert_eq!(claude.path, PathBuf::from(".claude"));
        let bar = claude.dirs.get("bar").expect("subdir");
        assert_eq!(bar.path, PathBuf::from(".claude/bar"));
    }

    #[test]
    fn aggregate_status_uniform() {
        let mut t = FileTree::new(PathBuf::new());
        t.insert(cf(".claude/a.md", FileStatus::Added));
        t.insert(cf(".claude/b/c.md", FileStatus::Added));
        let claude = t.dirs.get(".claude").unwrap();
        assert_eq!(aggregate_status(claude), Some(FileStatus::Added));
    }

    #[test]
    fn aggregate_status_mixed_returns_none() {
        let mut t = FileTree::new(PathBuf::new());
        t.insert(cf(".claude/a.md", FileStatus::Added));
        t.insert(cf(".claude/b.md", FileStatus::Modified));
        let claude = t.dirs.get(".claude").unwrap();
        assert_eq!(aggregate_status(claude), None);
    }

    #[test]
    fn flatten_collapsed_hides_children() {
        let mut t = FileTree::new(PathBuf::new());
        t.insert(cf(".claude/a.md", FileStatus::Added));
        t.insert(cf(".claude/b.md", FileStatus::Added));
        t.insert(cf("main.rs", FileStatus::Modified));

        let expanded = HashSet::new();
        let mut rows = Vec::new();
        flatten_file_tree(&t, 0, &expanded, &mut rows);
        // One collapsed dir row + one root file row.
        assert_eq!(rows.len(), 2);
        assert!(matches!(rows[0], FileTreeRow::Dir { .. }));
        assert!(matches!(rows[1], FileTreeRow::File { .. }));
    }

    #[test]
    fn flatten_expanded_reveals_children() {
        let mut t = FileTree::new(PathBuf::new());
        t.insert(cf(".claude/a.md", FileStatus::Added));
        t.insert(cf(".claude/b.md", FileStatus::Added));

        let mut expanded = HashSet::new();
        expanded.insert(".claude".to_string());
        let mut rows = Vec::new();
        flatten_file_tree(&t, 0, &expanded, &mut rows);
        // Dir row + two file rows at depth 1.
        assert_eq!(rows.len(), 3);
        match &rows[1] {
            FileTreeRow::File { depth, .. } => assert_eq!(*depth, 1),
            _ => panic!("expected file row"),
        }
    }
}
