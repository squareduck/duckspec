//! Change area — single change workspace with three-column layout.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use iced::widget::{button, column, container, row, scrollable, svg, text, Space};
use iced::widget::text::Wrapping;
use iced::{Element, Length};

use crate::data::{ChangeData, ProjectData, StepCompletion};
use crate::theme;
use crate::vcs::{self, ChangedFile, FileStatus};
use crate::widget::{collapsible, interaction_toggle, tab_bar, tree_view};

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

const ICON_SIZE: f32 = 14.0;

// ── State ────────────────────────────────────────────────────────────────────

pub struct State {
    pub selected_change: Option<String>,
    pub expanded_sections: HashSet<String>,
    pub expanded_nodes: HashSet<String>,
    pub tabs: tab_bar::TabState,
    pub changed_files: Vec<ChangedFile>,
    /// Per-change interaction states keyed by change name.
    /// Switching changes keeps previous sessions alive.
    pub interactions: HashMap<String, InteractionState>,
    /// Virtual exploration changes (not persisted to duckspec).
    pub explorations: Vec<String>,
    /// Counter for generating unique exploration names.
    pub exploration_counter: usize,
    /// When true, content column shows the audit view.
    pub audit_active: bool,
}

impl State {
    pub fn new(project_root: Option<&Path>) -> Self {
        let mut sections = HashSet::new();
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
            tabs: tab_bar::TabState::default(),
            changed_files: vec![],
            interactions: HashMap::new(),
            explorations,
            exploration_counter,
            audit_active: false,
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
        crate::chat_store::rename_scope(old_name, archived_name, project_root);
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
    TabContent(tab_bar::TabContentMsg),
    AddExploration,
    RemoveExploration(String),
    ShowAudit,
    RefreshAudit,
    SelectAuditError { change: String, artifact_id: String },
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
            state.audit_active = false;
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
                    let Some(ix) = state.active_interaction_mut() else { return };
                    interaction::ensure_sessions(ix, &scope, project.project_root.as_deref(), highlighter);
                    let new_session = AgentSession::new(scope.clone());
                    let _ = crate::chat_store::save_session(&new_session.session, project.project_root.as_deref());
                    ix.sessions.insert(0, new_session);
                    ix.active_session = 0;
                    interaction::reconcile_display_names(&mut ix.sessions);
                }
                interaction::Msg::SelectSession(id) => {
                    let Some(ix) = state.active_interaction_mut() else { return };
                    if let Some(idx) = ix.find_session_index(&id) {
                        ix.active_session = idx;
                    }
                }
                interaction::Msg::ClearSession => {
                    // Multi-session areas don't surface a Clear button, but
                    // handle it defensively by resetting the active session.
                    let Some(ix) = state.active_interaction_mut() else { return };
                    clear_active_session(ix, &scope, project.project_root.as_deref());
                }
                other => {
                    let Some(ix) = state.active_interaction_mut() else { return };
                    interaction::update_with_side_effects(
                        ix, other, &scope,
                        project.project_root.as_deref(), highlighter,
                    );
                }
            }
        }
        Message::SelectChangedFile(path) => {
            state.audit_active = false;
            if let Some(root) = &project.project_root
                && let Some(diff) = vcs::file_diff(root, &path) {
                    let id = format!("vcs:{}", path.display());
                    let title = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| path.display().to_string());

                    let ext = path
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("txt");
                    let syntax = highlighter.find_syntax(ext);
                    let old_lines: Vec<String> = diff.old_content.lines().map(String::from).collect();
                    let new_lines: Vec<String> = diff.new_content.lines().map(String::from).collect();
                    let highlight = crate::widget::diff_view::DiffHighlight {
                        old_spans: highlighter.highlight_lines(&old_lines, syntax),
                        new_spans: highlighter.highlight_lines(&new_lines, syntax),
                    };

                    let diff_status = diff.status;
                    let diff_path = diff.path.clone();
                    let editor = crate::widget::diff_view::build_editor(&diff, Some(&highlight));
                    state.tabs.open_diff(id, title, editor, diff_path, diff_status);
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
            crate::chat_store::save_explorations(&state.explorations, state.exploration_counter, project.project_root.as_deref());
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
            crate::chat_store::save_explorations(&state.explorations, state.exploration_counter, project.project_root.as_deref());
        }
        Message::ShowAudit => {
            state.audit_active = true;
            state.selected_change = None;
        }
        Message::RefreshAudit => {
            // Handled by main.rs — calls project.revalidate().
        }
        Message::SelectAuditError { change, artifact_id } => {
            state.audit_active = false;
            state.selected_change = Some(change.clone());
            state.expanded_nodes.clear();
            if let Some(ch) = project
                .active_changes
                .iter()
                .chain(project.archived_changes.iter())
                .find(|c| c.name == change)
            {
                crate::data::TreeNode::collect_parent_ids(
                    &ch.cap_tree,
                    &mut state.expanded_nodes,
                );
            }
            open_artifact(state, &artifact_id, project, highlighter);
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
        return Some(if all_done { "ds-archive".into() } else { "ds-apply".into() });
    }

    // Caps exist → feature flow needs steps next; refinement/doc-only is ready to archive.
    if !change.cap_tree.is_empty() {
        return Some(if change.has_design { "ds-step".into() } else { "ds-archive".into() });
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
    let Some(name) = state.selected_change.clone() else { return };
    let Some(ix) = state.interactions.get_mut(&name) else { return };
    for ax in ix.sessions.iter_mut() {
        ax.obvious_command = cmd.clone();
    }
}

// ── View ─────────────────────────────────────────────────────────────────────

pub fn view<'a>(
    state: &'a State,
    project: &'a ProjectData,
) -> Element<'a, Message> {
    let list = view_list(state, project);
    let divider = container(Space::new().height(Length::Fill))
        .width(1.0)
        .style(theme::divider);

    let is_exploration = state.is_exploration_selected();
    let ix = state.active_interaction();

    let breadcrumb_bar = view_breadcrumbs(breadcrumbs(state, project));
    let bar_divider = container(Space::new().width(Length::Fill))
        .height(1.0)
        .style(theme::divider);

    // Exploration mode: no content column or toggle, interaction fills remaining width.
    if is_exploration {
        let mut main_row = row![
            container(list)
                .width(theme::LIST_COLUMN_WIDTH)
                .height(Length::Fill)
                .style(theme::surface),
            divider,
        ];

        if let Some(ix) = ix {
            let interaction_col = interaction::view_column(ix, Message::Interaction, SessionControls::Single);
            main_row = main_row.push(
                container(interaction_col)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(theme::surface),
            );
        }

        return column![breadcrumb_bar, bar_divider, main_row.height(Length::Fill)]
            .height(Length::Fill)
            .into();
    }

    // Normal mode: content column + optional interaction panel.
    let content = if state.audit_active {
        view_audit(project)
    } else {
        view_content(state, project)
    };
    let visible = ix.is_some_and(|i| i.visible);
    let width = ix.map_or(theme::INTERACTION_COLUMN_WIDTH, |i| i.width);

    let toggle =
        interaction_toggle::view(visible, width, |m| Message::Interaction(interaction::Msg::Handle(m)));

    let mut main_row = row![
        container(list)
            .width(theme::LIST_COLUMN_WIDTH)
            .height(Length::Fill)
            .style(theme::surface),
        divider,
        container(content)
            .width(Length::Fill)
            .height(Length::Fill),
        toggle,
    ];

    if let Some(ix) = ix
        && ix.visible {
            let interaction_col = interaction::view_column(ix, Message::Interaction, SessionControls::Multi);

            main_row = main_row.push(
                container(interaction_col)
                    .width(ix.width)
                    .height(Length::Fill)
                    .style(theme::surface),
            );
        }

    column![breadcrumb_bar, bar_divider, main_row.height(Length::Fill)]
        .height(Length::Fill)
        .into()
}

// ── Breadcrumbs ──────────────────────────────────────────────────────────────

fn breadcrumbs(state: &State, project: &ProjectData) -> Vec<String> {
    let Some(selected) = state.selected_change.as_deref() else {
        return vec![];
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
        let root = if selected_archived { "Archive" } else { "Changes" };
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

fn view_breadcrumbs(segments: Vec<String>) -> Element<'static, Message> {
    let mut bar = row![].spacing(theme::SPACING_XS);
    let last = segments.len().saturating_sub(1);
    for (i, seg) in segments.into_iter().enumerate() {
        if i > 0 {
            bar = bar.push(
                text("\u{203a}")
                    .size(theme::font_sm())
                    .color(theme::text_muted()),
            );
        }
        let color = if i == last {
            theme::text_primary()
        } else {
            theme::text_muted()
        };
        bar = bar.push(
            text(seg)
                .size(theme::font_sm())
                .wrapping(Wrapping::None)
                .color(color),
        );
    }
    container(bar)
        .padding([theme::SPACING_XS, theme::SPACING_LG])
        .width(Length::Fill)
        .style(theme::surface)
        .into()
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
            tab_breadcrumbs("archive/2026-04-20-01-foo/proposal.md", "2026-04-20-01-foo", true),
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
            vec!["Archive", "2026-04-20-01-foo", "Changed files", "src/main.rs"]
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
            tabs: tab_bar::TabState::default(),
            changed_files: vec![],
            interactions: HashMap::new(),
            explorations: explorations.iter().map(|s| s.to_string()).collect(),
            exploration_counter: 0,
            audit_active: false,
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
        assert_eq!(breadcrumbs(&state, &project), vec!["Explorations", "Exploration 1"]);
    }

    #[test]
    fn exploration_promoted_to_change_shows_changes_root() {
        // After promote_exploration: selected_change → new name,
        // explorations list no longer contains the old name,
        // active_changes contains the new name.
        let state = make_state("real-change", &[]);
        let project = make_project(&["real-change"], &[]);
        assert_eq!(breadcrumbs(&state, &project), vec!["Changes", "real-change"]);
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
        crate::data::TreeNode { id: id.into(), label: id.into(), children: vec![] }
    }

    fn step(done: bool) -> crate::data::StepInfo {
        crate::data::StepInfo {
            id: "changes/foo/steps/01-bar.md".into(),
            label: "bar".into(),
            number: 1,
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
            tabs: tab_bar::TabState::default(),
            changed_files: vec![],
            interactions: HashMap::new(),
            explorations: vec![],
            exploration_counter: 0,
            audit_active: false,
        };
        let project = make_project(&[], &[]);
        assert_eq!(compute_obvious_command(&state, &project), None);
    }

    #[test]
    fn obvious_exploration_always_explore() {
        let state = make_state("Exploration 1", &["Exploration 1"]);
        let project = make_project(&[], &[]);
        assert_eq!(compute_obvious_command(&state, &project).as_deref(), Some("ds-explore"));
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
        assert_eq!(compute_obvious_command(&state, &project).as_deref(), Some("ds-propose"));
    }

    #[test]
    fn obvious_with_proposal_suggests_design() {
        let state = make_state("foo", &[]);
        let mut project = make_project(&["foo"], &[]);
        set_change(&mut project, "foo", |c| c.has_proposal = true);
        assert_eq!(compute_obvious_command(&state, &project).as_deref(), Some("ds-design"));
    }

    #[test]
    fn obvious_with_design_suggests_spec() {
        let state = make_state("foo", &[]);
        let mut project = make_project(&["foo"], &[]);
        set_change(&mut project, "foo", |c| {
            c.has_proposal = true;
            c.has_design = true;
        });
        assert_eq!(compute_obvious_command(&state, &project).as_deref(), Some("ds-spec"));
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
        assert_eq!(compute_obvious_command(&state, &project).as_deref(), Some("ds-step"));
    }

    #[test]
    fn obvious_refinement_with_caps_suggests_archive() {
        // Spec refinement / doc-only: caps present but no design.
        let state = make_state("foo", &[]);
        let mut project = make_project(&["foo"], &[]);
        set_change(&mut project, "foo", |c| {
            c.cap_tree = vec![tree_node("caps/auth")];
        });
        assert_eq!(compute_obvious_command(&state, &project).as_deref(), Some("ds-archive"));
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
        assert_eq!(compute_obvious_command(&state, &project).as_deref(), Some("ds-apply"));
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
        assert_eq!(compute_obvious_command(&state, &project).as_deref(), Some("ds-archive"));
    }
}

/// Reset the active session for a scope: cancel agent, delete persisted file,
/// and replace with a fresh empty session under a new id.
fn clear_active_session(
    ix: &mut InteractionState,
    scope: &str,
    project_root: Option<&Path>,
) {
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
        crate::chat_store::delete_session(
            &ax.session.scope,
            &ax.session.id,
            project_root,
        );
    }
    ix.sessions[idx] = AgentSession::new(scope.to_string());
    ix.active_session = idx;
    interaction::reconcile_display_names(&mut ix.sessions);
}

fn view_list<'a>(state: &'a State, project: &'a ProjectData) -> Element<'a, Message> {
    let mut selector = column![].spacing(theme::SPACING_XS);

    // Exploration changes (virtual) — listed first.
    for name in &state.explorations {
        let is_selected = state.selected_change.as_deref() == Some(name);
        let style = if is_selected {
            theme::list_item_active as fn(&iced::Theme, button::Status) -> button::Style
        } else {
            theme::list_item
        };
        let icon = svg(svg::Handle::from_memory(ICON_EXPLORE))
            .width(ICON_SIZE)
            .height(ICON_SIZE)
            .style(theme::svg_tint(theme::text_muted()));
        let close_btn = button(text("\u{00d7}").size(theme::font_md()))
            .on_press(Message::RemoveExploration(name.clone()))
            .padding(0.0)
            .style(theme::icon_button);
        let label = row![
            icon,
            text(name).size(theme::font_md()).wrapping(Wrapping::None),
            Space::new().width(Length::Fill),
            close_btn,
        ]
        .spacing(theme::SPACING_XS)
        .align_y(iced::Center);
        selector = selector.push(
            button(label)
                .on_press(Message::SelectChange(name.clone()))
                .width(Length::Fill)
                .padding([theme::SPACING_XS, theme::SPACING_SM])
                .style(style),
        );
    }

    // Real changes from duckspec.
    let all_changes: Vec<_> = project
        .active_changes
        .iter()
        .chain(project.archived_changes.iter())
        .collect();

    for ch in &all_changes {
        let is_selected = state
            .selected_change
            .as_ref() == Some(&ch.name);
        let style = if is_selected {
            theme::list_item_active as fn(&iced::Theme, button::Status) -> button::Style
        } else {
            theme::list_item
        };
        let icon = svg(svg::Handle::from_memory(ICON_BRANCH))
            .width(ICON_SIZE)
            .height(ICON_SIZE)
            .style(theme::svg_tint(theme::text_muted()));
        let mut label = row![icon, text(&ch.name).size(theme::font_md()).wrapping(Wrapping::None)]
            .spacing(theme::SPACING_XS)
            .align_y(iced::Center);
        if let Some(v) = project.validations.get(&ch.name) {
            let count = v.total_count();
            if count > 0 {
                label = label.push(Space::new().width(Length::Fill));
                label = label.push(
                    text(count.to_string())
                        .size(theme::font_sm())
                        .color(theme::error()),
                );
            }
        }
        selector = selector.push(
            button(label)
                .on_press(Message::SelectChange(ch.name.clone()))
                .width(Length::Fill)
                .padding([theme::SPACING_XS, theme::SPACING_SM])
                .style(style),
        );
    }

    let change_section = {
        let arrow = if state.expanded_sections.contains("picker") { "\u{25bf}" } else { "\u{25b9}" };
        let header = button(
            row![
                text("CHANGE")
                    .size(theme::font_sm())
                    .color(theme::text_secondary()),
                Space::new().width(Length::Fill),
                text(arrow).size(theme::font_sm()).color(theme::text_muted()),
            ]
            .width(Length::Fill),
        )
        .on_press(Message::ToggleSection("picker".to_string()))
        .width(Length::Fill)
        .style(theme::section_header)
        .padding([theme::SPACING_SM, theme::SPACING_SM]);

        let mut col = column![
            row![
                container(header).width(Length::Fill),
                button(text("+").size(theme::font_sm()).color(theme::text_secondary()))
                    .on_press(Message::AddExploration)
                    .padding([theme::SPACING_SM, theme::SPACING_SM])
                    .style(theme::section_header),
            ]
        ].spacing(0.0);

        if state.expanded_sections.contains("picker") {
            col = col.push(selector);
        }
        col
    };

    // Audit header — always visible.
    let audit_section = {
        let total_errors: usize = project.validations.values().map(|v| v.total_count()).sum();
        let title_color = if state.audit_active {
            theme::accent()
        } else {
            theme::text_secondary()
        };
        let style = if state.audit_active {
            theme::list_item_active as fn(&iced::Theme, button::Status) -> button::Style
        } else {
            theme::section_header
        };
        let mut label = row![
            text("AUDIT")
                .size(theme::font_sm())
                .color(title_color),
        ]
        .width(Length::Fill);
        if total_errors > 0 {
            label = label.push(Space::new().width(Length::Fill));
            label = label.push(
                text(total_errors.to_string())
                    .size(theme::font_sm())
                    .color(theme::error()),
            );
        }
        button(label)
            .on_press(Message::ShowAudit)
            .width(Length::Fill)
            .style(style)
            .padding([theme::SPACING_SM, theme::SPACING_SM])
    };

    let change = find_change(state, project);
    let is_exploration = state.is_exploration_selected();
    let mut list_col = column![audit_section, change_section].spacing(0.0);

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
    } else if !state.audit_active {
        if let Some(change) = change {
            let error_ids: HashSet<String> = project
                .validations
                .get(&change.name)
                .map(|v| v.file_errors.iter().map(|(p, _)| p.clone()).collect())
                .unwrap_or_default();
            list_col = list_col.push(view_overview_section(state, change, &error_ids));
            list_col = list_col.push(view_caps_section(state, change, &error_ids));
            list_col = list_col.push(view_steps_section(state, change, &error_ids));
        }
    }

    if !state.audit_active && change.is_none() && !is_exploration {
        list_col = list_col.push(
            container(
                text("Select a change")
                    .size(theme::font_md())
                    .color(theme::text_muted()),
            )
            .padding([theme::SPACING_SM, theme::SPACING_SM]),
        );
    }

    // Changed files section (always visible, independent of selected change).
    list_col = list_col.push(view_changed_files_section(state));

    scrollable(list_col)
        .direction(theme::thin_scrollbar_direction())
        .style(theme::thin_scrollbar)
        .height(Length::Fill)
        .into()
}

fn view_overview_section<'a>(
    state: &'a State,
    change: &'a ChangeData,
    error_ids: &HashSet<String>,
) -> Element<'a, Message> {
    let mut items = column![].spacing(theme::SPACING_XS);

    if change.has_proposal {
        let id = format!("{}/proposal.md", change.prefix);
        let has_err = error_ids.contains(&id);
        items = items.push(file_item("proposal.md", &id, has_err, state));
    }
    if change.has_design {
        let id = format!("{}/design.md", change.prefix);
        let has_err = error_ids.contains(&id);
        items = items.push(file_item("design.md", &id, has_err, state));
    }
    if !change.has_proposal && !change.has_design {
        items = items.push(
            container(text("No overview files").size(theme::font_md()).color(theme::text_muted()))
                .padding([2.0, theme::SPACING_SM]),
        );
    }

    collapsible::view(
        "Overview",
        state.expanded_sections.contains("overview"),
        Message::ToggleSection("overview".to_string()),
        items.into(),
    )
}

fn view_caps_section<'a>(
    state: &'a State,
    change: &'a ChangeData,
    error_ids: &HashSet<String>,
) -> Element<'a, Message> {
    let content = if change.cap_tree.is_empty() {
        container(text("No capability changes").size(theme::font_md()).color(theme::text_muted()))
            .padding([2.0, theme::SPACING_SM])
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
    let mut items = column![].spacing(theme::SPACING_XS);

    if change.steps.is_empty() {
        items = items.push(
            container(text("No steps").size(theme::font_md()).color(theme::text_muted()))
                .padding([2.0, theme::SPACING_SM]),
        );
    } else {
        for step in &change.steps {
            let is_active = state.tabs.active_tab().is_some_and(|t| t.id == step.id);
            let has_error = error_ids.contains(&step.id);
            let style = if is_active {
                theme::list_item_active as fn(&iced::Theme, button::Status) -> button::Style
            } else {
                theme::list_item
            };
            let icon_bytes: &[u8] = match step.completion {
                StepCompletion::Done => ICON_STEP_DONE,
                StepCompletion::Partial(_, _) => ICON_STEP_PARTIAL,
                StepCompletion::NoTasks => ICON_STEP,
            };
            let icon = svg(svg::Handle::from_memory(icon_bytes))
                .width(ICON_SIZE)
                .height(ICON_SIZE)
                .style(theme::svg_tint(theme::text_muted()));
            let mut label = row![
                icon,
                text(format!("{:02}-{}", step.number, step.label)).size(theme::font_md()).wrapping(Wrapping::None),
            ]
            .spacing(theme::SPACING_XS)
            .align_y(iced::Center);
            if has_error {
                label = label.push(Space::new().width(Length::Fill));
                label = label.push(
                    text("\u{2022}").size(theme::font_md()).color(theme::error()),
                );
            }
            items = items.push(
                button(label)
                    .on_press(Message::SelectItem(step.id.clone()))
                    .width(Length::Fill)
                    .padding([2.0, theme::SPACING_SM])
                    .style(style),
            );
        }
    }

    collapsible::view(
        "Steps",
        state.expanded_sections.contains("steps"),
        Message::ToggleSection("steps".to_string()),
        items.into(),
    )
}

fn view_changed_files_section<'a>(state: &'a State) -> Element<'a, Message> {
    let mut items = column![].spacing(theme::SPACING_XS);

    if state.changed_files.is_empty() {
        items = items.push(
            container(text("No changes").size(theme::font_md()).color(theme::text_muted()))
                .padding([2.0, theme::SPACING_SM]),
        );
    } else {
        for cf in &state.changed_files {
            let status_char = match cf.status {
                FileStatus::Modified => "M",
                FileStatus::Added => "A",
                FileStatus::Deleted => "D",
            };
            let color = theme::vcs_status_color(&cf.status);
            let tab_id = format!("vcs:{}", cf.path.display());
            let is_active = state.tabs.active_tab().is_some_and(|t| t.id == tab_id);
            let style = if is_active {
                theme::list_item_active as fn(&iced::Theme, button::Status) -> button::Style
            } else {
                theme::list_item
            };

            let label = row![
                text(status_char)
                    .size(theme::font_md())
                    .font(theme::content_font())
                    .color(color),
                text(cf.path.display().to_string()).size(theme::font_md()).wrapping(Wrapping::None),
            ]
            .spacing(theme::SPACING_SM);

            items = items.push(
                button(label)
                    .on_press(Message::SelectChangedFile(cf.path.clone()))
                    .width(Length::Fill)
                    .padding([2.0, theme::SPACING_SM])
                    .style(style),
            );
        }
    }

    collapsible::view(
        "Changed Files",
        state.expanded_sections.contains("changed_files"),
        Message::ToggleSection("changed_files".to_string()),
        items.into(),
    )
}

fn view_audit<'a>(project: &'a ProjectData) -> Element<'a, Message> {
    let total_errors: usize = project.validations.values().map(|v| v.total_count()).sum();

    // ── Header ─────────────────────────────────────────────────────────
    let summary = if total_errors == 0 {
        text("All checks passed")
            .size(theme::font_md())
            .color(theme::success())
    } else {
        text(format!(
            "{} error{}",
            total_errors,
            if total_errors == 1 { "" } else { "s" }
        ))
        .size(theme::font_md())
        .color(theme::error())
    };

    let header = container(
        row![
            column![
                text("Audit").size(22.0).color(theme::text_primary()),
                summary,
            ]
            .spacing(theme::SPACING_XS),
            Space::new().width(Length::Fill),
            button(
                text("Refresh")
                    .size(theme::font_md())
                    .color(theme::accent()),
            )
            .on_press(Message::RefreshAudit)
            .padding([theme::SPACING_SM, theme::SPACING_LG])
            .style(theme::dashboard_action),
        ]
        .align_y(iced::Center)
        .width(Length::Fill),
    )
    .padding([theme::SPACING_LG, theme::SPACING_XL]);

    let mut content = column![header].spacing(theme::SPACING_MD);

    if total_errors > 0 {
        let mut changes: Vec<_> = project.validations.iter().collect();
        changes.sort_by_key(|(name, _)| name.as_str());

        for (change_name, validation) in changes {
            content = content.push(view_audit_change(change_name, validation));
        }
    }

    scrollable(
        container(content)
            .padding([0.0, theme::SPACING_XL])
            .width(Length::Fill),
    )
    .direction(theme::thin_scrollbar_direction())
    .style(theme::thin_scrollbar)
    .height(Length::Fill)
    .width(Length::Fill)
    .into()
}

/// Render a single change's errors as a card.
fn view_audit_change<'a>(
    change_name: &'a str,
    validation: &'a crate::data::ChangeValidation,
) -> Element<'a, Message> {
    let icon = svg(svg::Handle::from_memory(ICON_BRANCH))
        .width(16.0)
        .height(16.0)
        .style(theme::svg_tint(theme::text_muted()));
    let change_header = row![
        icon,
        text(change_name).size(15.0).color(theme::text_primary()),
        Space::new().width(Length::Fill),
        text(format!(
            "{} error{}",
            validation.total_count(),
            if validation.total_count() == 1 { "" } else { "s" }
        ))
        .size(theme::font_sm())
        .color(theme::text_muted()),
    ]
    .spacing(theme::SPACING_SM)
    .align_y(iced::Center);

    let mut card = column![change_header].spacing(theme::SPACING_MD);

    // Per-file error groups.
    for (path, errors) in &validation.file_errors {
        let artifact_id = path.clone();
        let change = change_name.to_string();
        let file_link = button(
            text(path.as_str())
                .size(theme::font_md())
                .color(theme::accent()),
        )
        .on_press(Message::SelectAuditError {
            change,
            artifact_id,
        })
        .padding(0.0)
        .style(theme::link_button);

        let mut error_list = column![].spacing(theme::SPACING_XS);
        for err in errors {
            error_list = error_list.push(
                text(err.as_str())
                    .size(theme::font_md())
                    .color(theme::error()),
            );
        }

        let group = column![
            file_link,
            container(error_list).padding([0.0, theme::SPACING_LG]),
        ]
        .spacing(theme::SPACING_XS);

        card = card.push(group);
    }

    // Cross-file change-level errors.
    if !validation.change_errors.is_empty() {
        let mut structural = column![
            text("Structural").size(theme::font_md()).color(theme::text_secondary()),
        ]
        .spacing(theme::SPACING_XS);

        for err in &validation.change_errors {
            structural = structural.push(
                container(
                    text(err.as_str())
                        .size(theme::font_md())
                        .color(theme::error()),
                )
                .padding([0.0, theme::SPACING_LG]),
            );
        }
        card = card.push(structural);
    }

    container(card)
        .padding(theme::SPACING_LG)
        .width(Length::Fill)
        .style(theme::audit_card)
        .into()
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

fn file_item<'a>(label: &str, id: &str, has_error: bool, state: &State) -> Element<'a, Message> {
    let is_active = state.tabs.active_tab().is_some_and(|t| t.id == id);
    let style = if is_active {
        theme::list_item_active as fn(&iced::Theme, button::Status) -> button::Style
    } else {
        theme::list_item
    };
    let icon = svg(svg::Handle::from_memory(icon_for_artifact(label)))
        .width(ICON_SIZE)
        .height(ICON_SIZE)
        .style(theme::svg_tint(theme::text_muted()));
    let mut content = row![icon, text(label.to_string()).size(theme::font_md()).wrapping(Wrapping::None)]
        .spacing(theme::SPACING_XS)
        .align_y(iced::Center);
    if has_error {
        content = content.push(Space::new().width(Length::Fill));
        content = content.push(
            text("\u{2022}").size(theme::font_md()).color(theme::error()),
        );
    }
    button(content)
        .on_press(Message::SelectItem(id.to_string()))
        .width(Length::Fill)
        .padding([2.0, theme::SPACING_SM])
        .style(style)
        .into()
}

fn view_content<'a>(state: &'a State, project: &'a ProjectData) -> Element<'a, Message> {
    let bar = tab_bar::view_bar(
        &state.tabs,
        Message::SelectTab,
        Message::CloseTab,
    );
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
        crate::open_artifact_tab(&mut state.tabs, id.to_string(), title, content, id, highlighter);
    }
}
