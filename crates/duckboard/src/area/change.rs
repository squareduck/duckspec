//! Change area — single change workspace with three-column layout.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use iced::Element;
use iced::widget::{Space, button, column, container, row, text};

use crate::chat_store::Exploration;
use crate::data::{ChangeData, ProjectData, StepCompletion, TreeNode};
use crate::scope::{Scope, ScopeKind};
use crate::theme;
use crate::vcs::{ChangedFile, FileStatus};
use crate::widget::list_view::{self, ListRow};
use crate::widget::{collapsible, tab_bar, tree_view, vertical_scroll};

use super::interaction::{self, AgentSession, InteractionState};

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
const ICON_IDEAS: &[u8] = include_bytes!("../../assets/icon_idea.svg");

/// Section key for the Files explorer in `expanded_sections`. Absent by
/// default — the explorer starts collapsed with its header pinned to the
/// bottom of the list column.
pub const FILES_SECTION: &str = "files";

/// Container id wrapping the list column's scroll viewport. Measured by the
/// scroll-into-view operation in `main::reveal_active_file_in_explorer`.
pub const EXPLORER_VIEWPORT_ID: &str = "change-list-viewport";

/// Container id wrapping the Files explorer's row block (uniform-height
/// rows). Measured together with `EXPLORER_VIEWPORT_ID` to derive a row's
/// position inside the scroll content.
pub const EXPLORER_CONTENT_ID: &str = "files-explorer-content";

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
    pub changed_files: Vec<ChangedFile>,
    /// Full project file tree (gitignore-respecting, hidden files excluded)
    /// shown in the Files explorer section. Dir node ids are root-relative
    /// paths; file node ids are `file:<rel-path>` so they match file tab ids
    /// and row highlighting derives directly from the active tab.
    pub explorer_tree: Vec<TreeNode>,
    /// Directory node ids expanded in the Files explorer tree.
    pub expanded_explorer_dirs: HashSet<String>,
    /// Virtual exploration changes (not persisted to duckspec). Each carries
    /// a stable `id` used as the on-disk scope key plus a mutable
    /// `display_name` the UI shows.
    pub explorations: Vec<Exploration>,
    /// Counter for seeding default exploration display names.
    pub exploration_counter: usize,
    /// Id of the exploration row currently under the cursor, if any. When
    /// set, the exploration row's icon slot renders a close button instead.
    pub hovered_exploration: Option<String>,
    /// Id of the exploration whose close button has been clicked once and
    /// is now "armed" — the next click commits the destructive delete.
    /// Cleared on hover-leave, on a different selection, on `AddExploration`,
    /// and whenever the destructive delete actually fires. Skipped entirely
    /// for explorations whose `session_count` is zero (nothing to lose).
    pub armed_remove_exploration: Option<String>,
    /// Vertical scroll offset for the list column.
    pub list_scroll: f32,
    /// Folder-slug → originating exploration id, recorded when an exploration
    /// session's agent runs `ds create change`. Consumed by
    /// `reload_and_reconcile` to attribute the new folder to the session that
    /// created it. Not persisted; not cleaned up (changes are infrequent).
    pub pending_bindings: HashMap<String, String>,
}

impl State {
    pub fn new(project_root: Option<&Path>) -> Self {
        let mut sections = HashSet::new();
        sections.insert("picker".to_string());
        sections.insert("overview".to_string());
        sections.insert("capabilities".to_string());
        sections.insert("reviews".to_string());
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
            changed_files: vec![],
            explorer_tree: vec![],
            expanded_explorer_dirs: HashSet::new(),
            explorations,
            exploration_counter,
            hovered_exploration: None,
            armed_remove_exploration: None,
            list_scroll: 0.0,
            pending_bindings: HashMap::new(),
        }
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

    /// Replace the Files explorer contents from a fresh project walk.
    /// `files` are root-relative paths. Expanded state is pruned to
    /// directories that still exist; everything stays collapsed by default.
    pub fn set_project_files(&mut self, files: &[PathBuf]) {
        let mut dirs = HashSet::new();
        self.explorer_tree = build_explorer_tree(files, &mut dirs);
        self.expanded_explorer_dirs.retain(|d| dirs.contains(d));
    }

    /// Expand every ancestor directory of a root-relative file path in the
    /// Files explorer, so the file's row is present in the flattened tree.
    pub fn expand_explorer_ancestors(&mut self, rel: &str) {
        let Some((dirs, _file)) = rel.rsplit_once('/') else {
            return;
        };
        let mut prefix = String::new();
        for part in dirs.split('/') {
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(part);
            self.expanded_explorer_dirs.insert(prefix.clone());
        }
    }

    /// Index of `target_id` among the visible (flattened) explorer rows,
    /// plus the total visible row count. Mirrors `tree_view`'s flatten
    /// order: a node counts one row, children only when their parent is
    /// expanded.
    pub fn explorer_flat_position(&self, target_id: &str) -> Option<(usize, usize)> {
        fn walk(
            nodes: &[TreeNode],
            expanded: &HashSet<String>,
            target: &str,
            index: &mut usize,
            found: &mut Option<usize>,
        ) {
            for node in nodes {
                if node.id == target {
                    *found = Some(*index);
                }
                *index += 1;
                if expanded.contains(&node.id) {
                    walk(&node.children, expanded, target, index, found);
                }
            }
        }
        let mut index = 0;
        let mut found = None;
        walk(
            &self.explorer_tree,
            &self.expanded_explorer_dirs,
            target_id,
            &mut index,
            &mut found,
        );
        found.map(|f| (f, index))
    }

    /// Whether the currently selected change is an exploration (virtual).
    pub fn is_exploration_selected(&self) -> bool {
        self.selected_change
            .as_deref()
            .is_some_and(|id| self.explorations.iter().any(|e| e.id == id))
    }

    /// Classify a scope key: is it an exploration or a real change?
    pub fn scope_kind_for(&self, scope: &str) -> ScopeKind {
        if self.explorations.iter().any(|e| e.id == scope) {
            ScopeKind::Exploration
        } else {
            ScopeKind::Change
        }
    }

    /// Build a `Scope` from a raw scope key, classifying via `explorations`.
    pub fn scope_for(&self, scope: &str) -> Scope {
        if self.explorations.iter().any(|e| e.id == scope) {
            Scope::Exploration(scope.to_string())
        } else {
            Scope::Change(scope.to_string())
        }
    }

    /// Human-readable label for a scope: exploration display_name if the
    /// scope is an exploration id, else the scope key itself.
    pub fn scope_display_label(&self, scope: &str) -> String {
        self.explorations
            .iter()
            .find(|e| e.id == scope)
            .map(|e| e.display_name.clone())
            .unwrap_or_else(|| scope.to_string())
    }
}

/// Build the Files explorer tree from root-relative paths. Directories
/// come first (sorted), then files (sorted), matching the changed-files
/// tree. Every directory id encountered is also collected into `dirs_out`
/// so the caller can prune stale expanded state.
fn build_explorer_tree(files: &[PathBuf], dirs_out: &mut HashSet<String>) -> Vec<TreeNode> {
    #[derive(Default)]
    struct Dir {
        dirs: BTreeMap<String, Dir>,
        files: Vec<String>,
    }

    let mut root = Dir::default();
    for path in files {
        let parts: Vec<&str> = path
            .components()
            .filter_map(|c| c.as_os_str().to_str())
            .collect();
        let Some((file_name, dir_parts)) = parts.split_last() else {
            continue;
        };
        let mut node = &mut root;
        for part in dir_parts {
            node = node.dirs.entry((*part).to_string()).or_default();
        }
        node.files.push((*file_name).to_string());
    }

    fn convert(dir: Dir, prefix: &str, dirs_out: &mut HashSet<String>) -> Vec<TreeNode> {
        let join = |name: &str| {
            if prefix.is_empty() {
                name.to_string()
            } else {
                format!("{prefix}/{name}")
            }
        };
        let mut nodes = Vec::new();
        for (name, sub) in dir.dirs {
            let path = join(&name);
            dirs_out.insert(path.clone());
            let children = convert(sub, &path, dirs_out);
            nodes.push(TreeNode {
                id: path,
                label: name,
                children,
            });
        }
        let mut files = dir.files;
        files.sort_unstable();
        for name in files {
            nodes.push(TreeNode {
                id: format!("file:{}", join(&name)),
                label: name,
                children: vec![],
            });
        }
        nodes
    }

    convert(root, "", dirs_out)
}

/// Directories the changed-files tree should leave collapsed even on first
/// appearance. The duckspec root is usually noise — the user is typically
/// looking at the project's own changes, not their tracked spec edits — but
/// can still be expanded by hand when wanted.
fn is_collapse_by_default(dir: &str) -> bool {
    dir == "duckspec"
}

/// Promote an exploration to a real change: remove from explorations list,
/// migrate interaction state and chat sessions from the exploration's id
/// scope to the new change name.
pub fn promote_exploration(
    state: &mut State,
    interactions: &mut HashMap<Scope, InteractionState>,
    exploration_id: &str,
    real_name: &str,
    project_root: Option<&Path>,
) {
    // Flush-before-mutate: persist every session this exploration holds before
    // its in-memory state is migrated, so an in-flight turn can't be lost by
    // the promotion.
    if let Some(ix) = interactions.get(&Scope::Exploration(exploration_id.to_string())) {
        interaction::flush_sessions(ix, project_root);
    }
    state.explorations.retain(|e| e.id != exploration_id);
    if let Some(mut ix) = interactions.remove(&Scope::Exploration(exploration_id.to_string())) {
        for ax in ix.sessions.iter_mut() {
            ax.session.scope = real_name.to_string();
            ax.scope_kind = ScopeKind::Change;
        }
        let target = Scope::Change(real_name.to_string());
        if let Some(existing) = interactions.get_mut(&target) {
            // Target scope is already live — fold the exploration's sessions in
            // rather than overwrite, preserving the target's subscriptions.
            interaction::merge_sessions(existing, ix.sessions, real_name);
        } else {
            interaction::reconcile_display_names(&mut ix.sessions, real_name);
            interactions.insert(target, ix);
        }
    }
    if state.selected_change.as_deref() == Some(exploration_id) {
        state.selected_change = Some(real_name.to_string());
    }
    crate::chat_store::merge_scope(exploration_id, real_name, project_root);
    crate::chat_store::save_explorations(
        &state.explorations,
        state.exploration_counter,
        project_root,
    );
}

/// Migrate interaction state and chat sessions from a change that was just
/// archived externally (via CLI, agent tool, etc.) to its new archived name.
pub fn archive_change(
    state: &mut State,
    interactions: &mut HashMap<Scope, InteractionState>,
    tabs: &mut tab_bar::TabState,
    old_name: &str,
    archived_name: &str,
    project_root: Option<&Path>,
) {
    if let Some(mut ix) = interactions.remove(&Scope::Change(old_name.to_string())) {
        for ax in ix.sessions.iter_mut() {
            ax.session.scope = archived_name.to_string();
        }
        interaction::reconcile_display_names(&mut ix.sessions, archived_name);
        interactions.insert(Scope::Change(archived_name.to_string()), ix);
    }
    if state.selected_change.as_deref() == Some(old_name) {
        state.selected_change = Some(archived_name.to_string());
    }
    rewrite_tab_ids_for_archive(tabs, old_name, archived_name);
    crate::chat_store::rename_scope(old_name, archived_name, project_root);
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
    Interaction(interaction::Msg),
    SelectChangedFile(PathBuf),
    ToggleFileDir(String),
    /// Toggle a directory node in the Files explorer tree.
    ToggleExplorerDir(String),
    /// A file row in the Files explorer was clicked. Payload is the row's
    /// node id (`file:<rel-path>`). Intercepted by `main::update`, which
    /// opens the file as a regular file tab.
    SelectExplorerFile(String),
    AddExploration,
    /// Soft-archive a live exploration (stamp `archived_at`, keep chats).
    ArchiveExploration(String),
    /// First click on the close button of an exploration that has chat
    /// sessions. Sets `armed_remove_exploration` so the next
    /// `RemoveExploration` for the same id commits.
    ArmRemoveExploration(String),
    RemoveExploration(String),
    HoverExploration(String),
    /// Payload is the exploration name the row thinks it's clearing. Only
    /// clear the hover state if it still matches — otherwise a stale exit
    /// from row N can wipe a fresh enter from row N+1 when both fire in
    /// the same event dispatch.
    UnhoverExploration(String),
    /// Navigate to a change and open one of its artifacts.
    OpenArtifact {
        change: String,
        artifact_id: String,
    },
    /// Navigate to the idea linked to a given change. Handled by the main
    /// loop (switches `active_area` to Ideas and selects the idea); a no-op
    /// here so the message body can be a plain String.
    OpenIdeaForChange(String),
    /// `+` on the Changed Files header → open the new-file modal seeded with
    /// the project root. Intercepted by `main::update`.
    AddFile,
    ScrollList(f32),
}

// ── Update ───────────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)] // area update: state + tabs + interaction + project + flags
pub fn update(
    state: &mut State,
    tabs: &mut tab_bar::TabState,
    interactions: &mut HashMap<Scope, InteractionState>,
    message: Message,
    project: &ProjectData,
    highlighter: &crate::highlight::SyntaxHighlighter,
    agent_input_hints: bool,
    window_w: f32,
) {
    match message {
        Message::SelectChange(name) => {
            state.selected_change = Some(name.clone());
            state.expanded_nodes.clear();
            state.armed_remove_exploration = None;

            let is_exploration = state.explorations.iter().any(|e| e.id == name);
            if !is_exploration
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

            // Reveal the row in the list when navigation arrives from
            // another area (the toolbar Change button on an idea, etc.) —
            // the archived section is collapsed by default, so a fresh
            // selection there would otherwise be invisible.
            let in_archived = project.archived_changes.iter().any(|c| c.name == name)
                || state
                    .explorations
                    .iter()
                    .any(|e| e.id == name && e.is_archived());
            let section = if in_archived { "archived" } else { "picker" };
            state.expanded_sections.insert(section.to_string());

            let kind = state.scope_kind_for(&name);
            let label = state.scope_display_label(&name);
            let scope = state.scope_for(&name);
            let ix = interactions.entry(scope).or_default();
            interaction::ensure_sessions_with_label(
                ix,
                &name,
                &label,
                kind,
                project.project_root.as_deref(),
                highlighter,
            );
            if !ix.visible {
                ix.visible = true;
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
        Message::ToggleExplorerDir(id) => {
            if !state.expanded_explorer_dirs.remove(&id) {
                state.expanded_explorer_dirs.insert(id);
            }
        }
        Message::SelectExplorerFile(_) => {
            // Intercepted by `main::update` (needs `open_path_in_tab`).
        }
        Message::SelectItem(id) => {
            open_artifact(tabs, &id, project, highlighter);
        }
        Message::Interaction(msg) => {
            let scope_key = match state.selected_change.clone() {
                Some(n) => n,
                None => return,
            };
            let kind = state.scope_kind_for(&scope_key);
            let label = state.scope_display_label(&scope_key);
            let scope = state.scope_for(&scope_key);
            match msg {
                interaction::Msg::NewSession => {
                    let ix = interactions.entry(scope.clone()).or_default();
                    interaction::ensure_sessions_with_label(
                        ix,
                        &scope_key,
                        &label,
                        kind,
                        project.project_root.as_deref(),
                        highlighter,
                    );
                    // Donor is still active — inherit next actions before insert.
                    let new_session = interaction::new_session_with_inherited_next_actions(
                        ix,
                        scope_key.clone(),
                        kind,
                    );
                    let _ = crate::chat_store::save_session(
                        &new_session.session,
                        project.project_root.as_deref(),
                    );
                    ix.sessions.insert(0, new_session);
                    ix.active_session = 0;
                    interaction::reconcile_display_names(&mut ix.sessions, &label);
                }
                interaction::Msg::SelectSession(id) => {
                    let Some(ix) = interactions.get_mut(&scope) else {
                        return;
                    };
                    if let Some(idx) = ix.find_session_index(&id) {
                        ix.active_session = idx;
                    }
                }
                interaction::Msg::ClearSession => {
                    let Some(ix) = interactions.get_mut(&scope) else {
                        return;
                    };
                    clear_active_session(
                        ix,
                        &scope_key,
                        &label,
                        kind,
                        project.project_root.as_deref(),
                    );
                }
                other => {
                    let Some(ix) = interactions.get_mut(&scope) else {
                        return;
                    };
                    interaction::update_with_side_effects(
                        ix,
                        other,
                        &scope_key,
                        &label,
                        kind,
                        project.project_root.as_deref(),
                        highlighter,
                        agent_input_hints,
                        window_w,
                    );
                }
            }
        }
        Message::SelectChangedFile(_) => {
            // Intercepted by `main::update` so the async diff-highlight
            // `Task` can be propagated to the runtime.
        }
        Message::AddExploration => {
            state.exploration_counter += 1;
            let exp = Exploration::new(state.exploration_counter);
            let id = exp.id.clone();
            let display_name = exp.display_name.clone();
            state.explorations.push(exp);
            state.selected_change = Some(id.clone());
            state.armed_remove_exploration = None;
            crate::chat_store::save_explorations(
                &state.explorations,
                state.exploration_counter,
                project.project_root.as_deref(),
            );
            let ix = interactions
                .entry(Scope::Exploration(id.clone()))
                .or_default();
            interaction::ensure_sessions_with_label(
                ix,
                &id,
                &display_name,
                ScopeKind::Exploration,
                project.project_root.as_deref(),
                highlighter,
            );
            ix.visible = true;
            crate::chat_store::recount_explorations(
                &mut state.explorations,
                project.project_root.as_deref(),
            );
        }
        Message::ArchiveExploration(id) => {
            if let Some(exp) = state.explorations.iter_mut().find(|e| e.id == id) {
                exp.mark_archived();
            }
            if state.armed_remove_exploration.as_deref() == Some(id.as_str()) {
                state.armed_remove_exploration = None;
            }
            crate::chat_store::save_explorations(
                &state.explorations,
                state.exploration_counter,
                project.project_root.as_deref(),
            );
        }
        Message::ArmRemoveExploration(id) => {
            state.armed_remove_exploration = Some(id);
        }
        Message::RemoveExploration(id) => {
            state.explorations.retain(|e| e.id != id);
            interactions.remove(&Scope::Exploration(id.clone()));
            if state.selected_change.as_deref() == Some(&id) {
                state.selected_change = None;
            }
            if state.hovered_exploration.as_deref() == Some(&id) {
                state.hovered_exploration = None;
            }
            if state.armed_remove_exploration.as_deref() == Some(&id) {
                state.armed_remove_exploration = None;
            }
            crate::chat_store::delete_scope(&id, project.project_root.as_deref());
            crate::chat_store::save_explorations(
                &state.explorations,
                state.exploration_counter,
                project.project_root.as_deref(),
            );
        }
        Message::HoverExploration(id) => {
            state.hovered_exploration = Some(id);
        }
        Message::UnhoverExploration(id) => {
            if state.hovered_exploration.as_deref() == Some(id.as_str()) {
                state.hovered_exploration = None;
            }
            // Moving the cursor off an armed row disarms it — matches the
            // visual disappearance of the red icon.
            if state.armed_remove_exploration.as_deref() == Some(id.as_str()) {
                state.armed_remove_exploration = None;
            }
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
            open_artifact(tabs, &artifact_id, project, highlighter);
        }
        Message::OpenIdeaForChange(_) => {
            // Handled in main.rs — crosses area boundaries.
        }
        Message::AddFile => {
            // Handled in main.rs — opens the global new-file modal.
        }
        Message::ScrollList(offset) => {
            state.list_scroll = offset;
        }
    }

    let vcs_dirty = !state.changed_files.is_empty();
    refresh_fast_response(interactions, project, agent_input_hints, vcs_dirty);
    // Cheap: one `read_dir` per exploration. Keeps `Exploration.session_count`
    // in sync so the close-button arming logic doesn't `read_dir` per frame.
    crate::chat_store::recount_explorations(
        &mut state.explorations,
        project.project_root.as_deref(),
    );
}

/// Compute the suggested next /ds-* command (without the leading slash) given
/// the selected change's artifact state. Returns `None` for archived changes
/// or when nothing is selected. Test-only — production paths refresh
/// `scope_facts` / next-action bootstrap via `refresh_fast_response`.
#[cfg(test)]
fn compute_lifecycle_command(state: &State, project: &ProjectData) -> Option<String> {
    let selected = state.selected_change.as_deref()?;

    // Exploration (virtual) — always orient first.
    if state.explorations.iter().any(|e| e.id == selected) {
        return Some("ds-explore".into());
    }

    lifecycle_command_from_artifacts(selected, project)
}

/// A change's lifecycle position, derived from its artifact and step state.
/// Drives both the fast response lifecycle list and the per-session scope
/// orientation blurb, so the two never disagree about where a change stands.
#[derive(Debug, Clone)]
pub struct ChangeScopeFacts {
    /// Human phase label, e.g. "specs drafted, steps not yet written".
    pub phase: &'static str,
    /// How many steps are complete, and how many there are. `step_count == 0`
    /// means the change has no steps yet.
    pub steps_done: usize,
    pub step_count: usize,
    /// Task tally `(done, total)` for the one in-progress (`Partial`) step, if
    /// any. `StepCompletion::Done` does not carry its total, so a full task
    /// aggregate is not recoverable — this reports only the active step.
    pub active_step_tasks: Option<(usize, usize)>,
    /// First lifecycle option (bare name) — orientation + oneshot soft hint.
    pub next_command: Option<String>,
    /// The change's current review — the highest-numbered review filename
    /// (`NN-<slug>.md`), or `None` when the change has no reviews. Surfaced in
    /// orientation; when present, also steers the review-aware lifecycle arms.
    pub current_review: Option<String>,
}

fn scope_facts(
    phase: &'static str,
    steps_done: usize,
    step_count: usize,
    active_step_tasks: Option<(usize, usize)>,
    lifecycle: &[&str],
    current_review: Option<String>,
) -> ChangeScopeFacts {
    let next_command = lifecycle.first().map(|s| (*s).to_string());
    ChangeScopeFacts {
        phase,
        steps_done,
        step_count,
        active_step_tasks,
        next_command,
        current_review,
    }
}

/// Inspect a change directory's artifact and step state and return its
/// lifecycle facts. Pure function over `project` — independent of
/// `state.selected_change`, so it can describe any change session (e.g.
/// freshly-promoted idea sessions where the user is still in the Ideas
/// area and the Changes area's selection hasn't moved). Returns `None` for
/// archived or unknown changes, which have no next stage.
pub fn change_scope_facts(name: &str, project: &ProjectData) -> Option<ChangeScopeFacts> {
    if project.archived_changes.iter().any(|c| c.name == name) {
        return None;
    }

    let change = project.active_changes.iter().find(|c| c.name == name)?;

    // The current review is the highest-numbered review (reviews are sorted
    // ascending). Computed before the phase branches and set in every arm so
    // it surfaces at any lifecycle stage — including a pre-implementation
    // review under a proposal-only change. When present, review-aware arms
    // below take priority over the plain ladder.
    let current_review = change.reviews.last().cloned();
    let has_review = current_review.is_some();

    let steps_done = change
        .steps
        .iter()
        .filter(|s| matches!(s.completion, StepCompletion::Done))
        .count();
    let step_count = change.steps.len();
    let has_steps = step_count > 0;
    let all_done = has_steps && steps_done == step_count;
    let open = has_steps && !all_done;
    let active_step_tasks = change.steps.iter().find_map(|s| match s.completion {
        StepCompletion::Partial(done, total) => Some((done, total)),
        _ => None,
    });

    // Open steps: apply first; review and followup stay available (including
    // when a critique file already exists — re-critique mid-impl).
    if open {
        return Some(scope_facts(
            "implementing steps",
            steps_done,
            step_count,
            active_step_tasks,
            &["ds-apply", "ds-review", "ds-followup"],
            current_review,
        ));
    }

    // No open steps + review: rework + re-critique + archive.
    if has_review {
        return Some(scope_facts(
            if all_done {
                "all steps complete, review on file"
            } else {
                "review on file, no open steps"
            },
            steps_done,
            step_count,
            active_step_tasks,
            &[
                "ds-step",
                "ds-spec",
                "ds-review",
                "ds-followup",
                "ds-archive",
            ],
            current_review,
        ));
    }

    // All steps complete, no review → archive + both critique modes.
    if all_done {
        return Some(scope_facts(
            "all steps complete",
            steps_done,
            step_count,
            active_step_tasks,
            &["ds-archive", "ds-review", "ds-followup"],
            current_review,
        ));
    }

    // Caps on disk, no steps — step or no-code archive (no re-entry to ds-spec).
    if !change.cap_tree.is_empty() {
        return Some(scope_facts(
            "specs drafted, steps not yet written",
            0,
            0,
            None,
            &["ds-step", "ds-archive"],
            current_review,
        ));
    }

    // No caps yet — feature-flow ladder (design optional).
    if change.has_design {
        return Some(scope_facts(
            "design drafted, specs not yet written",
            0,
            0,
            None,
            &["ds-spec", "ds-step"],
            current_review,
        ));
    }
    if change.has_proposal {
        return Some(scope_facts(
            "proposal drafted, design not yet written",
            0,
            0,
            None,
            &["ds-design", "ds-spec"],
            current_review,
        ));
    }
    Some(scope_facts(
        "newly created, no artifacts yet",
        0,
        0,
        None,
        &["ds-propose"],
        current_review,
    ))
}

/// Suggested next `/ds-*` command for a change, derived from its lifecycle
/// facts. Thin caller over `change_scope_facts` so the placeholder and the
/// scope orientation share one source of truth. Production paths derive the
/// command from already-computed facts in `refresh_fast_response`; this
/// wrapper exists for the test-only `compute_lifecycle_command`.
#[cfg(test)]
fn lifecycle_command_from_artifacts(name: &str, project: &ProjectData) -> Option<String> {
    change_scope_facts(name, project).and_then(|f| f.next_command)
}

/// Refresh `scope_facts` and re-sync fast-response chips on every session of
/// every change / exploration interaction. `scope_facts` drives orientation and
/// empty-session next-action bootstrap. Shell is re-derived from settled oneshot
/// when eligible; a parked user-choice fill is left alone.
pub fn refresh_fast_response(
    interactions: &mut HashMap<Scope, InteractionState>,
    project: &ProjectData,
    agent_input_hints: bool,
    _vcs_dirty: bool,
) {
    for (scope, ix) in interactions.iter_mut() {
        if matches!(scope, Scope::Caps | Scope::Codex) {
            continue;
        }
        // Facts once per change scope; non-change scopes carry none.
        let facts = match scope {
            Scope::Change(name) => change_scope_facts(name, project),
            Scope::Exploration(_) | Scope::Caps | Scope::Codex => None,
        };
        for ax in ix.sessions.iter_mut() {
            ax.scope_facts = facts.clone();
            // Next-action list uses scope_facts bootstrap and last assistant.
            // Not a turn boundary — keep Tab index if the list is unchanged.
            ax.refresh_next_actions(false);
            // Re-sync oneshot chips after next-actions (eligibility depends on
            // empty next-action list). Leaves UserChoice fills alone.
            super::interaction::sync_oneshot_chips(ax, agent_input_hints);
        }
    }
}

// ── Breadcrumbs ──────────────────────────────────────────────────────────────

pub fn breadcrumbs(state: &State, project: &ProjectData, tabs: &tab_bar::TabState) -> Vec<String> {
    let Some(selected) = state.selected_change.as_deref() else {
        return vec!["Changes".into()];
    };

    if let Some(exp) = state.explorations.iter().find(|e| e.id == selected) {
        return vec!["Explorations".into(), exp.display_name.clone()];
    }

    let is_archived = project.archived_changes.iter().any(|c| c.name == selected);

    if let Some(tab) = tabs.active_tab() {
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
    if let Some(rest) = path.strip_prefix("reviews/") {
        return vec!["Reviews".into(), rest.into()];
    }
    path.split('/').map(str::to_string).collect()
}

/// Hover leading-control action for an exploration row.
/// Live → soft archive (one click). Archived → remove with arm when sessions remain.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ExplorationHoverAction {
    Archive(String),
    ArmRemove(String),
    /// `armed` tints the control red (second click after arming).
    Remove { id: String, armed: bool },
}

fn exploration_hover_action(exp: &Exploration, armed: bool) -> ExplorationHoverAction {
    if !exp.is_archived() {
        ExplorationHoverAction::Archive(exp.id.clone())
    } else if exp.session_count == 0 {
        ExplorationHoverAction::Remove {
            id: exp.id.clone(),
            armed: false,
        }
    } else if armed {
        ExplorationHoverAction::Remove {
            id: exp.id.clone(),
            armed: true,
        }
    } else {
        ExplorationHoverAction::ArmRemove(exp.id.clone())
    }
}

fn exploration_hover_button<'a>(action: ExplorationHoverAction) -> Element<'a, Message> {
    match action {
        ExplorationHoverAction::Archive(id) => {
            collapsible::close_button_sized(Message::ArchiveExploration(id), list_view::ICON_SIZE)
        }
        ExplorationHoverAction::Remove { id, armed: true } => collapsible::close_button_sized_tinted(
            Message::RemoveExploration(id),
            list_view::ICON_SIZE,
            theme::error(),
        ),
        ExplorationHoverAction::Remove { id, armed: false } => {
            collapsible::close_button_sized(Message::RemoveExploration(id), list_view::ICON_SIZE)
        }
        ExplorationHoverAction::ArmRemove(id) => {
            collapsible::close_button_sized(Message::ArmRemoveExploration(id), list_view::ICON_SIZE)
        }
    }
}

/// Unified Archived list row (Change list + Dashboard).
#[derive(Debug, Clone, Copy)]
pub enum ArchivedEntry<'a> {
    Change(&'a ChangeData),
    Exploration(&'a Exploration),
}

impl ArchivedEntry<'_> {
    /// Sort key: higher is more recent (string-desc works for folder prefixes
    /// and ISO archive stamps when compared lexicographically).
    fn sort_key(self) -> String {
        match self {
            ArchivedEntry::Change(ch) => ch.name.clone(),
            ArchivedEntry::Exploration(exp) => exp.archived_at.clone().unwrap_or_default(),
        }
    }
}

/// Non–idea-owned archived explorations + archived changes, newest first.
pub fn archived_entries<'a>(
    changes: &'a [ChangeData],
    explorations: &'a [Exploration],
) -> Vec<ArchivedEntry<'a>> {
    let mut entries: Vec<ArchivedEntry<'a>> = changes
        .iter()
        .map(ArchivedEntry::Change)
        .chain(
            explorations
                .iter()
                .filter(|e| e.is_on_archived_list())
                .map(ArchivedEntry::Exploration),
        )
        .collect();
    entries.sort_by_key(|e| std::cmp::Reverse(e.sort_key()));
    entries
}

pub fn has_archived_section(changes: &[ChangeData], explorations: &[Exploration]) -> bool {
    !changes.is_empty() || explorations.iter().any(|e| e.is_on_archived_list())
}

/// Rows under the Change picker: live explorations + active changes.
fn change_section_count(explorations: &[Exploration], active: &[ChangeData]) -> usize {
    explorations.iter().filter(|e| e.is_on_live_list()).count() + active.len()
}

/// Rows under Archived: interleaved archived entries length.
#[cfg(test)]
fn archived_section_count(changes: &[ChangeData], explorations: &[Exploration]) -> usize {
    archived_entries(changes, explorations).len()
}

// ── View ─────────────────────────────────────────────────────────────────────

pub fn view_list<'a>(
    state: &'a State,
    project: &'a ProjectData,
    ideas: &'a super::ideas::State,
    tabs: &'a tab_bar::TabState,
) -> Element<'a, Message> {
    let mut rows: Vec<ListRow<'a, Message>> = vec![];

    // Live explorations (virtual) — listed first. Hidden when idea-owned or archived.
    for exp in state.explorations.iter().filter(|e| e.is_on_live_list()) {
        let is_selected = state.selected_change.as_deref() == Some(exp.id.as_str());
        let is_hovered = state.hovered_exploration.as_deref() == Some(exp.id.as_str());
        let mut r = ListRow::new(exp.display_name.as_str())
            .selected(is_selected)
            .on_press(Message::SelectChange(exp.id.clone()))
            .on_hover(
                Message::HoverExploration(exp.id.clone()),
                Message::UnhoverExploration(exp.id.clone()),
            );
        if is_hovered {
            let armed = state.armed_remove_exploration.as_deref() == Some(exp.id.as_str());
            let action = exploration_hover_action(exp, armed);
            let close = exploration_hover_button(action);
            r = r.leading(close);
        } else {
            r = r.icon(ICON_EXPLORE);
        }
        rows.push(r);
    }

    for ch in &project.active_changes {
        let is_selected = state.selected_change.as_ref() == Some(&ch.name);
        let has_err = project
            .validations
            .get(&ch.name)
            .is_some_and(|v| v.total_count() > 0);
        let mut r = ListRow::new(ch.name.as_str())
            .icon(ICON_BRANCH)
            .selected(is_selected)
            .errored(has_err)
            .on_press(Message::SelectChange(ch.name.clone()));
        if ideas.idea_path_for_change(&ch.name).is_some() {
            r = r.after_icon(idea_link_button(&ch.name));
        }
        rows.push(r);
    }

    let change_count = change_section_count(&state.explorations, &project.active_changes);
    debug_assert_eq!(rows.len(), change_count);
    let selector = list_view::view(rows, None);

    let archived_list = archived_entries(&project.archived_changes, &state.explorations);
    let archived_count = archived_list.len();
    let archived_rows: Vec<ListRow<'a, Message>> = archived_list
        .into_iter()
        .map(|entry| match entry {
            ArchivedEntry::Change(ch) => {
                let is_selected = state.selected_change.as_ref() == Some(&ch.name);
                let has_err = project
                    .validations
                    .get(&ch.name)
                    .is_some_and(|v| v.total_count() > 0);
                let base = crate::data::strip_archive_prefix(&ch.name).unwrap_or(&ch.name);
                let mut r = ListRow::new(ch.name.as_str())
                    .icon(ICON_BRANCH)
                    .selected(is_selected)
                    .errored(has_err)
                    .on_press(Message::SelectChange(ch.name.clone()));
                if ideas.idea_path_for_change(base).is_some() {
                    r = r.after_icon(idea_link_button(base));
                }
                r
            }
            ArchivedEntry::Exploration(exp) => {
                let is_selected = state.selected_change.as_deref() == Some(exp.id.as_str());
                let is_hovered = state.hovered_exploration.as_deref() == Some(exp.id.as_str());
                let mut r = ListRow::new(exp.display_name.as_str())
                    .selected(is_selected)
                    .on_press(Message::SelectChange(exp.id.clone()))
                    .on_hover(
                        Message::HoverExploration(exp.id.clone()),
                        Message::UnhoverExploration(exp.id.clone()),
                    );
                if is_hovered {
                    let armed = state.armed_remove_exploration.as_deref() == Some(exp.id.as_str());
                    r = r.leading(exploration_hover_button(exploration_hover_action(
                        exp, armed,
                    )));
                } else {
                    r = r.icon(ICON_EXPLORE);
                }
                r
            }
        })
        .collect();

    let archived_section =
        if has_archived_section(&project.archived_changes, &state.explorations) {
            Some(collapsible::view_with_add_owned(
                format!("Archived  ({archived_count})"),
                state.expanded_sections.contains("archived"),
                Message::ToggleSection("archived".to_string()),
                None,
                list_view::view(archived_rows, None),
            ))
        } else {
            None
        };

    let change_section = collapsible::view_with_add_owned(
        format!("Change  ({change_count})"),
        state.expanded_sections.contains("picker"),
        Message::ToggleSection("picker".to_string()),
        Some(collapsible::add_button(Message::AddExploration)),
        selector,
    );

    let change = find_change(state, project);
    let is_exploration = state.is_exploration_selected();
    let mut list_col = column![change_section].spacing(0.0);

    if let Some(section) = archived_section {
        list_col = list_col.push(section);
    }

    if is_exploration {
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
        list_col = list_col.push(view_overview_section(tabs, state, change, &error_ids));
        list_col = list_col.push(view_caps_section(tabs, state, change, &error_ids));
        list_col = list_col.push(view_reviews_section(tabs, state, change, &error_ids));
        list_col = list_col.push(view_steps_section(tabs, state, change, &error_ids));
    }

    list_col = list_col.push(view_changed_files_section(tabs, state));

    let files_expanded = state.expanded_sections.contains(FILES_SECTION);
    if files_expanded {
        list_col = list_col.push(view_files_section(tabs, state));
    }

    let scroll: Element<'a, Message> = container(vertical_scroll::view(
        state.list_scroll,
        Message::ScrollList,
        list_col,
    ))
    .width(iced::Length::Fill)
    .height(iced::Length::Fill)
    .id(EXPLORER_VIEWPORT_ID)
    .into();

    if files_expanded {
        scroll
    } else {
        // Collapsed: pin the Files header below the scroll viewport so it
        // stays anchored to the bottom edge of the list column regardless
        // of scroll position.
        column![scroll, view_files_section(tabs, state)]
            .height(iced::Length::Fill)
            .into()
    }
}

fn view_overview_section<'a>(
    tabs: &'a tab_bar::TabState,
    state: &'a State,
    change: &'a ChangeData,
    error_ids: &HashSet<String>,
) -> Element<'a, Message> {
    let active_id = tabs.active_tab().map(|t| t.id.as_str());
    let mut rows: Vec<ListRow<'a, Message>> = vec![];

    let mut push_file = |label: &'static str, id: String, has_err: bool| {
        let r = ListRow::new(label)
            .icon(icon_for_artifact(label))
            .selected(active_id == Some(id.as_str()))
            .errored(has_err)
            .on_press(Message::SelectItem(id));
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
    tabs: &'a tab_bar::TabState,
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
            tabs.active_tab().map(|t| t.id.as_str()),
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

fn view_reviews_section<'a>(
    tabs: &'a tab_bar::TabState,
    state: &'a State,
    change: &'a ChangeData,
    error_ids: &HashSet<String>,
) -> Element<'a, Message> {
    let active_id = tabs.active_tab().map(|t| t.id.as_str());
    let rows: Vec<ListRow<'a, Message>> = change
        .reviews
        .iter()
        .map(|filename| {
            let id = format!("{}/reviews/{}", change.prefix, filename);
            let has_err = error_ids.contains(&id);
            ListRow::new(filename.as_str())
                .icon(ICON_DOC)
                .selected(active_id == Some(id.as_str()))
                .errored(has_err)
                .on_press(Message::SelectItem(id))
        })
        .collect();

    collapsible::view(
        "Reviews",
        state.expanded_sections.contains("reviews"),
        Message::ToggleSection("reviews".to_string()),
        list_view::view(rows, Some("No reviews")),
    )
}

fn view_steps_section<'a>(
    tabs: &'a tab_bar::TabState,
    state: &'a State,
    change: &'a ChangeData,
    error_ids: &HashSet<String>,
) -> Element<'a, Message> {
    let active_id = tabs.active_tab().map(|t| t.id.as_str());
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
            let has_err = error_ids.contains(&step.id);
            let mut r = ListRow::new(step.label.as_str())
                .icon(icon_bytes)
                .selected(active_id == Some(step.id.as_str()))
                .errored(has_err)
                .on_press(Message::SelectItem(step.id.clone()));
            if let Some(tint) = icon_tint {
                r = r.icon_tint(tint);
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
struct FileTree {
    dirs: BTreeMap<String, FileTree>,
    files: Vec<ChangedFile>,
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

fn view_changed_files_section<'a>(
    tabs: &'a tab_bar::TabState,
    state: &'a State,
) -> Element<'a, Message> {
    let rows: Vec<ListRow<'a, Message>> = if state.changed_files.is_empty() {
        vec![]
    } else {
        let mut tree = FileTree::new(PathBuf::new());
        for cf in &state.changed_files {
            tree.insert(cf.clone());
        }
        let mut flat = Vec::new();
        flatten_file_tree(&tree, 0, &state.expanded_file_dirs, &mut flat);

        let active_tab_id = tabs.active_tab().map(|t| t.id.as_str());

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

    collapsible::view_with_add(
        "Changed Files",
        state.expanded_sections.contains("changed_files"),
        Message::ToggleSection("changed_files".to_string()),
        Some(collapsible::add_button(Message::AddFile)),
        list_view::view(rows, Some("No changes")),
    )
}

/// Files explorer section — the full project tree, gitignore-respecting.
/// File rows carry `file:<rel>` node ids, so the row of the file open in
/// the content column highlights without any extra selection state.
fn view_files_section<'a>(tabs: &'a tab_bar::TabState, state: &'a State) -> Element<'a, Message> {
    let expanded = state.expanded_sections.contains(FILES_SECTION);

    let content: Element<'a, Message> = if !expanded {
        // Collapsible sections skip their content when collapsed.
        Space::new().into()
    } else if state.explorer_tree.is_empty() {
        container(
            text("No files")
                .size(theme::font_md())
                .color(theme::text_muted()),
        )
        .padding([theme::SPACING_XS, theme::SPACING_SM])
        .into()
    } else {
        let no_errors = HashSet::new();
        let tints = explorer_vcs_tints(&state.changed_files);
        container(tree_view::view_with_tints(
            &state.explorer_tree,
            &state.expanded_explorer_dirs,
            tabs.active_tab().map(|t| t.id.as_str()),
            &no_errors,
            &tints,
            Message::ToggleExplorerDir,
            Message::SelectExplorerFile,
        ))
        .id(EXPLORER_CONTENT_ID)
        .into()
    };

    collapsible::view_with_add(
        "Files",
        expanded,
        Message::ToggleSection(FILES_SECTION.to_string()),
        Some(collapsible::add_button(Message::AddFile)),
        content,
    )
}

/// Per-node tints for the Files explorer: VCS-changed files take their
/// status color, and every ancestor directory takes the aggregate color of
/// the changes inside it — falling back to the modified color when statuses
/// are mixed — so collapsed directories still signal where changes live.
/// Deleted files have no row in the explorer (they're gone from the working
/// tree) but still tint their ancestors.
fn explorer_vcs_tints(changed: &[ChangedFile]) -> HashMap<String, iced::Color> {
    let mut tints = HashMap::new();
    let mut dir_status: HashMap<String, Option<FileStatus>> = HashMap::new();
    for cf in changed {
        let parts: Vec<&str> = cf
            .path
            .components()
            .filter_map(|c| c.as_os_str().to_str())
            .collect();
        if parts.is_empty() {
            continue;
        }
        if cf.status != FileStatus::Deleted {
            tints.insert(
                format!("file:{}", parts.join("/")),
                theme::vcs_status_color(&cf.status),
            );
        }
        let mut prefix = String::new();
        for part in &parts[..parts.len() - 1] {
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(part);
            dir_status
                .entry(prefix.clone())
                .and_modify(|s| {
                    if *s != Some(cf.status) {
                        *s = None;
                    }
                })
                .or_insert(Some(cf.status));
        }
    }
    for (dir, status) in dir_status {
        let color = theme::vcs_status_color(&status.unwrap_or(FileStatus::Modified));
        tints.insert(dir, color);
    }
    tints
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

fn idea_link_button<'a>(change_name: &str) -> Element<'a, Message> {
    let icon = iced::widget::svg(iced::widget::svg::Handle::from_memory(ICON_IDEAS))
        .width(list_view::ICON_SIZE)
        .height(list_view::ICON_SIZE)
        .style(theme::svg_tint(theme::accent()));
    button(icon)
        .on_press(Message::OpenIdeaForChange(change_name.to_string()))
        .padding(0.0)
        .style(theme::icon_button)
        .into()
}

/// Errors associated with the active artifact tab. Used by main.rs's content
/// column renderer to draw the error panel below the editor.
pub fn error_panel_for<'a>(
    state: &State,
    project: &'a ProjectData,
    tabs: &tab_bar::TabState,
) -> Option<&'a [String]> {
    let tab = tabs.active_tab()?;
    let change_name = state.selected_change.as_ref()?;
    let validation = project.validations.get(change_name)?;
    validation
        .file_errors
        .iter()
        .find(|(path, _)| *path == tab.id)
        .map(|(_, errs)| errs.as_slice())
        .filter(|errs| !errs.is_empty())
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
    tabs: &mut tab_bar::TabState,
    id: &str,
    project: &ProjectData,
    highlighter: &crate::highlight::SyntaxHighlighter,
) {
    if let Some(content) = project.read_artifact(id) {
        let title = id.rsplit('/').next().unwrap_or(id).to_string();
        let path = project.duckspec_root.as_ref().map(|r| r.join(id));
        crate::open_artifact_tab(tabs, id.to_string(), title, content, id, path, highlighter);
    }
}

/// Reset the active session for a scope: cancel agent, delete persisted file,
/// and replace with a fresh empty session under a new id.
fn clear_active_session(
    ix: &mut InteractionState,
    scope: &str,
    scope_label: &str,
    scope_kind: ScopeKind,
    project_root: Option<&Path>,
) {
    if ix.sessions.is_empty() {
        ix.sessions
            .push(AgentSession::new(scope.to_string(), scope_kind));
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
    ix.sessions[idx] = AgentSession::new(scope.to_string(), scope_kind);
    ix.active_session = idx;
    interaction::reconcile_display_names(&mut ix.sessions, scope_label);
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

    fn make_state(selected: &str, explorations: &[(&str, &str)]) -> State {
        State {
            selected_change: Some(selected.to_string()),
            expanded_sections: HashSet::new(),
            expanded_nodes: HashSet::new(),
            expanded_file_dirs: HashSet::new(),
            changed_files: vec![],
            explorer_tree: vec![],
            expanded_explorer_dirs: HashSet::new(),
            explorations: explorations
                .iter()
                .map(|(id, name)| Exploration {
                    id: (*id).to_string(),
                    display_name: (*name).to_string(),
                    idea_path: None,
                    archived_at: None,
                    session_count: 0,
                })
                .collect(),
            exploration_counter: 0,
            hovered_exploration: None,
            armed_remove_exploration: None,
            list_scroll: 0.0,
            known_file_dirs: HashSet::new(),
            pending_bindings: HashMap::new(),
        }
    }

    fn make_project(active: &[&str], archived: &[&str]) -> ProjectData {
        let mk = |name: &str, prefix_root: &str| ChangeData {
            name: name.to_string(),
            prefix: format!("{prefix_root}/{name}"),
            has_proposal: false,
            has_design: false,
            cap_tree: vec![],
            steps: vec![],
            reviews: vec![],
        };
        ProjectData {
            active_changes: active.iter().map(|n| mk(n, "changes")).collect(),
            archived_changes: archived.iter().map(|n| mk(n, "archive")).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn exploration_root_after_selection() {
        let state = make_state("exploration-1000", &[("exploration-1000", "Exploration 1")]);
        let project = make_project(&[], &[]);
        let tabs = tab_bar::TabState::default();
        assert_eq!(
            breadcrumbs(&state, &project, &tabs),
            vec!["Explorations", "Exploration 1"]
        );
    }

    #[test]
    fn exploration_promoted_to_change_shows_changes_root() {
        let state = make_state("real-change", &[]);
        let project = make_project(&["real-change"], &[]);
        let tabs = tab_bar::TabState::default();
        assert_eq!(
            breadcrumbs(&state, &project, &tabs),
            vec!["Changes", "real-change"]
        );
    }

    #[test]
    fn change_archived_shows_archive_root() {
        let state = make_state("2026-04-20-01-foo", &[]);
        let project = make_project(&[], &["2026-04-20-01-foo"]);
        let tabs = tab_bar::TabState::default();
        assert_eq!(
            breadcrumbs(&state, &project, &tabs),
            vec!["Archive", "2026-04-20-01-foo"]
        );
    }

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
    fn lifecycle_nothing_selected() {
        let state = State {
            selected_change: None,
            expanded_sections: HashSet::new(),
            expanded_nodes: HashSet::new(),
            expanded_file_dirs: HashSet::new(),
            changed_files: vec![],
            explorer_tree: vec![],
            expanded_explorer_dirs: HashSet::new(),
            explorations: vec![],
            exploration_counter: 0,
            hovered_exploration: None,
            armed_remove_exploration: None,
            list_scroll: 0.0,
            known_file_dirs: HashSet::new(),
            pending_bindings: HashMap::new(),
        };
        let project = make_project(&[], &[]);
        assert_eq!(compute_lifecycle_command(&state, &project), None);
    }

    #[test]
    fn lifecycle_exploration_always_explore() {
        let state = make_state("exploration-1000", &[("exploration-1000", "Exploration 1")]);
        let project = make_project(&[], &[]);
        assert_eq!(
            compute_lifecycle_command(&state, &project).as_deref(),
            Some("ds-explore")
        );
    }

    #[test]
    fn lifecycle_archived_is_none() {
        let state = make_state("2026-04-20-01-foo", &[]);
        let project = make_project(&[], &["2026-04-20-01-foo"]);
        assert_eq!(compute_lifecycle_command(&state, &project), None);
    }

    #[test]
    fn lifecycle_empty_change_suggests_propose() {
        let state = make_state("foo", &[]);
        let project = make_project(&["foo"], &[]);
        assert_eq!(
            compute_lifecycle_command(&state, &project).as_deref(),
            Some("ds-propose")
        );
    }

    #[test]
    fn lifecycle_with_proposal_suggests_design() {
        let state = make_state("foo", &[]);
        let mut project = make_project(&["foo"], &[]);
        set_change(&mut project, "foo", |c| c.has_proposal = true);
        assert_eq!(
            compute_lifecycle_command(&state, &project).as_deref(),
            Some("ds-design")
        );
    }

    #[test]
    fn lifecycle_with_design_suggests_spec() {
        let state = make_state("foo", &[]);
        let mut project = make_project(&["foo"], &[]);
        set_change(&mut project, "foo", |c| {
            c.has_proposal = true;
            c.has_design = true;
        });
        assert_eq!(
            compute_lifecycle_command(&state, &project).as_deref(),
            Some("ds-spec")
        );
    }

    #[test]
    fn lifecycle_feature_flow_with_caps_suggests_step() {
        let state = make_state("foo", &[]);
        let mut project = make_project(&["foo"], &[]);
        set_change(&mut project, "foo", |c| {
            c.has_proposal = true;
            c.has_design = true;
            c.cap_tree = vec![tree_node("caps/auth")];
        });
        assert_eq!(
            compute_lifecycle_command(&state, &project).as_deref(),
            Some("ds-step")
        );
    }

    #[test]
    fn lifecycle_caps_without_design_still_suggests_step() {
        // Design is optional: a feature change can go proposal → spec → step
        // with no design.md. Caps-but-no-steps must suggest `ds-step`, never
        // `ds-archive`, regardless of whether a design exists.
        let state = make_state("foo", &[]);
        let mut project = make_project(&["foo"], &[]);
        set_change(&mut project, "foo", |c| {
            c.has_proposal = true;
            c.cap_tree = vec![tree_node("caps/auth")];
        });
        assert_eq!(
            compute_lifecycle_command(&state, &project).as_deref(),
            Some("ds-step")
        );
    }

    #[test]
    fn lifecycle_steps_unfinished_suggests_apply() {
        let state = make_state("foo", &[]);
        let mut project = make_project(&["foo"], &[]);
        set_change(&mut project, "foo", |c| {
            c.has_proposal = true;
            c.has_design = true;
            c.cap_tree = vec![tree_node("caps/auth")];
            c.steps = vec![step(false), step(true)];
        });
        assert_eq!(
            compute_lifecycle_command(&state, &project).as_deref(),
            Some("ds-apply")
        );
    }

    fn make_tab(id: &str) -> crate::widget::tab_bar::Tab {
        crate::widget::tab_bar::Tab {
            id: id.into(),
            title: id.rsplit('/').next().unwrap_or(id).into(),
            view: crate::widget::tab_bar::TabView::Editor {
                editor: crate::widget::text_edit::EditorState::new(""),
                path: None,
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
        assert_eq!(tabs.file_tabs[1].id, "changes/bar/proposal.md");
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
    fn lifecycle_all_steps_done_suggests_archive() {
        let state = make_state("foo", &[]);
        let mut project = make_project(&["foo"], &[]);
        set_change(&mut project, "foo", |c| {
            c.has_proposal = true;
            c.has_design = true;
            c.cap_tree = vec![tree_node("caps/auth")];
            c.steps = vec![step(true), step(true)];
        });
        assert_eq!(
            compute_lifecycle_command(&state, &project).as_deref(),
            Some("ds-archive")
        );
    }

    // @spec chat/fast-response Population: Ordinary refresh leaves options empty when oneshot is ineligible
    #[test]
    fn ordinary_refresh_leaves_options_empty_when_oneshot_is_ineligible() {
        use crate::area::interaction::{AgentSession, InteractionState};
        use crate::scope::{Scope, ScopeKind};
        use std::collections::HashMap;

        // GIVEN not awaiting and oneshot ineligible (hints off / empty list)
        let mut project = make_project(&["foo"], &[]);
        set_change(&mut project, "foo", |c| {
            c.has_proposal = true;
            c.has_design = true;
            c.cap_tree = vec![tree_node("caps/auth")];
            c.steps = vec![step(false)];
        });
        let mut interactions = HashMap::new();
        let scope = Scope::Change("foo".into());
        let mut ix = InteractionState::default();
        let mut ax = AgentSession::new("foo".into(), ScopeKind::Change);
        // Non-empty session without trailing next → empty next-actions; still
        // ineligible without settled oneshot + hints.
        ax.session.messages.push(crate::chat_store::ChatMessage {
            role: crate::chat_store::Role::User,
            content: vec![crate::chat_store::ContentBlock::Text("hi".into())],
            timestamp: String::new(),
            is_priming: false,
        });
        ax.agent_default_prompts = vec!["would show if eligible".into()];
        ix.sessions.push(ax);
        interactions.insert(scope, ix);

        refresh_fast_response(&mut interactions, &project, false, false);

        let ax = interactions
            .get(&Scope::Change("foo".into()))
            .and_then(|i| i.sessions.first())
            .expect("session");
        assert!(ax.fast_response.options.is_empty());
    }

    // @spec chat/fast-response Population: Refresh does not clear options while awaiting a user choice
    #[test]
    fn refresh_does_not_clear_options_while_awaiting_a_user_choice() {
        use crate::area::interaction::{AgentSession, InteractionState};
        use crate::fast_response::{self, FastResponseSource};
        use crate::scope::{Scope, ScopeKind};
        use std::collections::HashMap;

        let mut project = make_project(&["foo"], &[]);
        set_change(&mut project, "foo", |c| {
            c.has_proposal = true;
            c.has_design = true;
            c.cap_tree = vec![tree_node("caps/auth")];
            c.steps = vec![step(false)];
        });
        let mut interactions = HashMap::new();
        let scope = Scope::Change("foo".into());
        let mut ix = InteractionState::default();
        let mut ax = AgentSession::new("foo".into(), ScopeKind::Change);
        ax.is_awaiting_user = true;
        ax.fast_response =
            fast_response::from_user_choice(99, [("a".into(), "Alpha".into())]);
        ix.sessions.push(ax);
        interactions.insert(scope, ix);

        refresh_fast_response(&mut interactions, &project, true, false);

        let ax = interactions
            .get(&Scope::Change("foo".into()))
            .and_then(|i| i.sessions.first())
            .expect("session");
        assert!(ax.is_awaiting_user);
        assert_eq!(ax.fast_response.options.len(), 1);
        assert!(matches!(
            ax.fast_response.source,
            FastResponseSource::UserChoice { correlation_id: 99 }
        ));
    }

    // @spec chat/fast-response Population: Refresh preserves oneshot fill when still eligible
    #[test]
    fn refresh_preserves_oneshot_fill_when_still_eligible() {
        use crate::area::interaction::{AgentSession, InteractionState};
        use crate::fast_response::{self, FastResponseSource};
        use crate::scope::{Scope, ScopeKind};
        use std::collections::HashMap;

        let project = make_project(&["foo"], &[]);
        let mut interactions = HashMap::new();
        let scope = Scope::Change("foo".into());
        let mut ix = InteractionState::default();
        let mut ax = AgentSession::new("foo".into(), ScopeKind::Change);
        // Non-empty session, no trailing next → empty next-actions.
        ax.session.messages.push(crate::chat_store::ChatMessage {
            role: crate::chat_store::Role::Assistant,
            content: vec![crate::chat_store::ContentBlock::Text("done".into())],
            timestamp: String::new(),
            is_priming: false,
        });
        ax.agent_default_prompts = vec!["most likely".into(), "alt".into()];
        ax.fast_response = fast_response::from_oneshot_hints(ax.agent_default_prompts.clone());
        ix.sessions.push(ax);
        interactions.insert(scope, ix);

        refresh_fast_response(&mut interactions, &project, true, false);

        let ax = interactions
            .get(&Scope::Change("foo".into()))
            .and_then(|i| i.sessions.first())
            .expect("session");
        assert_eq!(ax.fast_response.options.len(), 2);
        assert_eq!(ax.fast_response.options[0].label, "most likely");
        assert_eq!(ax.fast_response.options[1].label, "alt");
        assert!(matches!(
            ax.fast_response.source,
            FastResponseSource::OneshotHints
        ));
    }

    // @spec chat/fast-response Population: Settled eligible oneshot fills the option shell
    #[test]
    fn settled_eligible_oneshot_fills_the_option_shell() {
        use crate::area::interaction::{self, AgentSession};
        use crate::fast_response::FastResponseSource;
        use crate::scope::ScopeKind;

        let mut ax = AgentSession::new("foo".into(), ScopeKind::Change);
        ax.session.messages.push(crate::chat_store::ChatMessage {
            role: crate::chat_store::Role::Assistant,
            content: vec![crate::chat_store::ContentBlock::Text("done".into())],
            timestamp: String::new(),
            is_priming: false,
        });
        ax.agent_default_prompts = vec!["yes".into(), "no".into(), "maybe".into()];
        ax.refresh_next_actions(true);
        assert!(ax.next_actions.is_empty());

        interaction::sync_oneshot_chips(&mut ax, true);

        assert_eq!(ax.fast_response.options.len(), 3);
        assert_eq!(ax.fast_response.options[0].id, "yes");
        assert!(matches!(
            ax.fast_response.source,
            FastResponseSource::OneshotHints
        ));
    }

    // @spec chat/fast-response Population: Live user choice overwrites oneshot fill
    #[test]
    fn live_user_choice_overwrites_oneshot_fill() {
        use crate::area::interaction::{self, AgentSession};
        use crate::fast_response::{self, FastResponseSource};
        use crate::scope::ScopeKind;

        let mut ax = AgentSession::new("foo".into(), ScopeKind::Change);
        ax.fast_response =
            fast_response::from_oneshot_hints(vec!["oneshot a".into(), "oneshot b".into()]);
        assert!(matches!(
            ax.fast_response.source,
            FastResponseSource::OneshotHints
        ));

        interaction::apply_user_choice_request(
            &mut ax,
            7,
            None,
            vec![("opt-a".into(), "Alpha".into())],
            false,
        );

        assert_eq!(ax.fast_response.options.len(), 1);
        assert_eq!(ax.fast_response.options[0].id, "opt-a");
        assert!(matches!(
            ax.fast_response.source,
            FastResponseSource::UserChoice { correlation_id: 7 }
        ));
        assert!(ax.is_awaiting_user);
    }

    // @spec chat/fast-response Population: Oneshot settle does not replace a live user-choice fill
    #[test]
    fn oneshot_settle_does_not_replace_a_live_user_choice_fill() {
        use crate::area::interaction::{self, AgentSession};
        use crate::fast_response::{self, FastResponseSource};
        use crate::scope::ScopeKind;

        let mut ax = AgentSession::new("foo".into(), ScopeKind::Change);
        ax.is_awaiting_user = true;
        ax.fast_response =
            fast_response::from_user_choice(42, [("q1".into(), "Option one".into())]);
        ax.agent_default_prompts = vec!["would fill if not awaiting".into()];
        ax.next_actions.clear();

        interaction::sync_oneshot_chips(&mut ax, true);

        assert_eq!(ax.fast_response.options.len(), 1);
        assert_eq!(ax.fast_response.options[0].id, "q1");
        assert!(matches!(
            ax.fast_response.source,
            FastResponseSource::UserChoice { correlation_id: 42 }
        ));
    }

    /// @spec session/scope Lifecycle reflection: A change with unfinished steps reports remaining work and the apply next-stage
    #[test]
    fn facts_unfinished_steps_report_remaining_work_and_apply() {
        let mut project = make_project(&["foo"], &[]);
        set_change(&mut project, "foo", |c| {
            c.has_proposal = true;
            c.has_design = true;
            c.cap_tree = vec![tree_node("caps/auth")];
            c.steps = vec![step(false), step(true)];
        });
        let facts = change_scope_facts("foo", &project).expect("active change has facts");
        assert!(
            facts.steps_done < facts.step_count,
            "progress should not be complete"
        );
        assert_eq!(facts.next_command.as_deref(), Some("ds-apply"));
    }

    // @spec session/scope Lifecycle reflection: A change with all steps complete reports completion and the archive next-stage
    #[test]
    fn facts_all_steps_complete_report_completion_and_archive() {
        // GIVEN all steps complete AND no reviews
        let mut project = make_project(&["foo"], &[]);
        set_change(&mut project, "foo", |c| {
            c.has_proposal = true;
            c.has_design = true;
            c.cap_tree = vec![tree_node("caps/auth")];
            c.steps = vec![step(true), step(true)];
        });
        let facts = change_scope_facts("foo", &project).expect("active change has facts");
        assert_eq!(facts.steps_done, facts.step_count);
        assert!(facts.step_count > 0, "completion is over real steps");
        assert_eq!(facts.next_command.as_deref(), Some("ds-archive"));
    }

    // @spec session/scope Lifecycle reflection: All steps complete with a review suggests the step next-stage
    #[test]
    fn facts_all_steps_complete_with_review_suggests_step() {
        let mut project = make_project(&["foo"], &[]);
        set_change(&mut project, "foo", |c| {
            c.has_proposal = true;
            c.has_design = true;
            c.cap_tree = vec![tree_node("caps/auth")];
            c.steps = vec![step(true), step(true)];
            c.reviews = vec!["01-look.md".into()];
        });
        let facts = change_scope_facts("foo", &project).expect("active change has facts");
        assert_eq!(facts.next_command.as_deref(), Some("ds-step"));
    }

    /// @spec session/scope Lifecycle reflection: A change with only a proposal reports the design next-stage
    #[test]
    fn facts_proposal_only_reports_design() {
        let mut project = make_project(&["foo"], &[]);
        set_change(&mut project, "foo", |c| c.has_proposal = true);
        let facts = change_scope_facts("foo", &project).expect("active change has facts");
        assert_eq!(facts.next_command.as_deref(), Some("ds-design"));
    }

    /// Produce the first-turn orientation text for a change scope, exercising
    /// the full data → facts → render path the session hook uses in production.
    fn orientation_for(name: &str, project: &ProjectData) -> String {
        use duckchat::ContextHook;
        let scope = crate::scope::SessionScope {
            kind: crate::scope::ScopeKind::Change,
            scope_key: name.to_string(),
            change_facts: change_scope_facts(name, project),
        };
        crate::scope::CurrentScopeHook
            .compute(&scope)
            .expect("change scope always produces orientation")
            .text
    }

    /// @spec session/scope Current review in orientation: Orientation reports the highest-numbered review as the current review
    #[test]
    fn orientation_reports_highest_numbered_review() {
        let mut project = make_project(&["foo"], &[]);
        set_change(&mut project, "foo", |c| {
            c.has_proposal = true;
            c.reviews = vec!["01-initial.md".into(), "02-post-implementation.md".into()];
        });
        let facts = change_scope_facts("foo", &project).expect("active change has facts");
        assert_eq!(facts.current_review.as_deref(), Some("02-post-implementation.md"));

        let text = orientation_for("foo", &project);
        assert!(
            text.contains("duckspec/changes/foo/reviews/02-post-implementation.md"),
            "orientation must report the highest-numbered review at the full path: {text}"
        );
        assert!(
            !text.contains("reviews/01-initial.md"),
            "orientation must not report a lower-numbered review as current: {text}"
        );
    }

    // @spec session/scope Current review in orientation: A change with no reviews reports no current review
    #[test]
    fn orientation_with_no_reviews_reports_none() {
        let mut project = make_project(&["foo"], &[]);
        set_change(&mut project, "foo", |c| c.has_proposal = true);
        let facts = change_scope_facts("foo", &project).expect("active change has facts");
        assert_eq!(facts.current_review, None);

        let text = orientation_for("foo", &project);
        assert!(
            !text.contains("Current review:"),
            "orientation must not report a current review when none exist: {text}"
        );
    }

    // @spec session/scope Current review in orientation: Adding a review does not change reported step progress
    #[test]
    fn adding_a_review_does_not_change_reported_step_progress() {
        // Two changes with identical step completion; only `bar` has reviews.
        let mut project = make_project(&["foo", "bar"], &[]);
        for name in ["foo", "bar"] {
            set_change(&mut project, name, |c| {
                c.has_proposal = true;
                c.has_design = true;
                c.cap_tree = vec![tree_node("caps/auth")];
                c.steps = vec![step(true), step(false)];
            });
        }
        set_change(&mut project, "bar", |c| {
            c.reviews = vec!["01-a-look.md".into()];
        });

        let foo = change_scope_facts("foo", &project).expect("facts");
        let bar = change_scope_facts("bar", &project).expect("facts");
        assert_eq!(foo.steps_done, bar.steps_done);
        assert_eq!(foo.step_count, bar.step_count);
        assert_eq!(foo.steps_done, 1);
        assert_eq!(foo.step_count, 2);
    }

    // ── exploration archive / live lists ────────────────────────────────

    fn live_list_ids(exps: &[Exploration]) -> Vec<&str> {
        exps.iter()
            .filter(|e| e.is_on_live_list())
            .map(|e| e.id.as_str())
            .collect()
    }

    /// @spec exploration/archive Live list membership: Archived non–idea-owned exploration is absent from live lists
    #[test]
    fn archived_non_idea_owned_absent_from_live_lists() {
        let mut exp = Exploration::new(1);
        exp.mark_archived();
        assert!(!exp.is_on_live_list());
        assert!(live_list_ids(std::slice::from_ref(&exp)).is_empty());
    }

    /// @spec exploration/archive Live list membership: Live non–idea-owned exploration remains on live lists
    #[test]
    fn live_non_idea_owned_remains_on_live_lists() {
        let exp = Exploration::new(1);
        assert!(exp.is_on_live_list());
        assert_eq!(
            live_list_ids(std::slice::from_ref(&exp)),
            vec![exp.id.as_str()]
        );
    }

    /// @spec exploration/archive Hover control by state: Live exploration hover control archives
    #[test]
    fn live_exploration_hover_control_archives() {
        use crate::test_support::{FsTmp, with_home};

        let tmp = FsTmp::new();
        with_home(tmp.path(), || {
            let root = tmp.path().join("project");
            std::fs::create_dir_all(&root).unwrap();

            let exp = Exploration::new(1);
            let exp_id = exp.id.clone();
            let mut session = crate::chat_store::ChatSession::new(exp_id.clone());
            session.id = "1".into();
            crate::chat_store::save_session(&session, Some(&root)).unwrap();

            let mut state = make_state(&exp_id, &[]);
            state.explorations = vec![exp];
            let mut project = make_project(&[], &[]);
            project.project_root = Some(root.clone());
            let mut tabs = tab_bar::TabState::default();
            let mut interactions = HashMap::new();
            let hl = crate::highlight::SyntaxHighlighter::new();

            assert_eq!(
                exploration_hover_action(&state.explorations[0], false),
                ExplorationHoverAction::Archive(exp_id.clone())
            );

            update(
                &mut state,
                &mut tabs,
                &mut interactions,
                Message::ArchiveExploration(exp_id.clone()),
                &project,
                &hl,
                false,
                1200.0,
            );

            let exp = state.explorations.iter().find(|e| e.id == exp_id).unwrap();
            assert!(exp.is_archived());
            assert!(!exp.is_on_live_list());
            assert_eq!(crate::chat_store::count_sessions(&exp_id, Some(&root)), 1);
        });
    }

    /// @spec exploration/archive Hover control by state: Archived exploration hover control removes
    #[test]
    fn archived_exploration_hover_control_removes() {
        use crate::test_support::{FsTmp, with_home};

        let tmp = FsTmp::new();
        with_home(tmp.path(), || {
            let root = tmp.path().join("project");
            std::fs::create_dir_all(&root).unwrap();

            let mut exp = Exploration::new(1);
            let exp_id = exp.id.clone();
            exp.mark_archived();
            exp.session_count = 1;

            let mut session = crate::chat_store::ChatSession::new(exp_id.clone());
            session.id = "1".into();
            crate::chat_store::save_session(&session, Some(&root)).unwrap();

            assert!(matches!(
                exploration_hover_action(&exp, true),
                ExplorationHoverAction::Remove { armed: true, .. }
            ));

            let mut state = make_state(&exp_id, &[]);
            state.explorations = vec![exp];
            let mut project = make_project(&[], &[]);
            project.project_root = Some(root.clone());
            let mut tabs = tab_bar::TabState::default();
            let mut interactions = HashMap::new();
            let hl = crate::highlight::SyntaxHighlighter::new();

            update(
                &mut state,
                &mut tabs,
                &mut interactions,
                Message::RemoveExploration(exp_id.clone()),
                &project,
                &hl,
                false,
                1200.0,
            );

            assert!(state.explorations.iter().all(|e| e.id != exp_id));
            assert_eq!(crate::chat_store::count_sessions(&exp_id, Some(&root)), 0);
        });
    }

    /// @spec exploration/archive Hover control by state: Remove with sessions requires arm then commit
    #[test]
    fn remove_with_sessions_requires_arm_then_commit() {
        let mut exp = Exploration::new(1);
        exp.mark_archived();
        exp.session_count = 2;
        let id = exp.id.clone();

        // First activation only arms (control routes to ArmRemove).
        assert_eq!(
            exploration_hover_action(&exp, false),
            ExplorationHoverAction::ArmRemove(id.clone())
        );

        let mut state = make_state(&id, &[]);
        state.explorations = vec![exp.clone()];
        let project = make_project(&[], &[]);
        let mut tabs = tab_bar::TabState::default();
        let mut interactions = HashMap::new();
        let hl = crate::highlight::SyntaxHighlighter::new();

        update(
            &mut state,
            &mut tabs,
            &mut interactions,
            Message::ArmRemoveExploration(id.clone()),
            &project,
            &hl,
            false,
            1200.0,
        );
        assert_eq!(state.armed_remove_exploration.as_deref(), Some(id.as_str()));
        assert!(state.explorations.iter().any(|e| e.id == id));

        // Armed second activation routes to Remove.
        assert_eq!(
            exploration_hover_action(&exp, true),
            ExplorationHoverAction::Remove {
                id: id.clone(),
                armed: true,
            }
        );

        update(
            &mut state,
            &mut tabs,
            &mut interactions,
            Message::RemoveExploration(id.clone()),
            &project,
            &hl,
            false,
            1200.0,
        );
        assert!(state.explorations.iter().all(|e| e.id != id));
    }

    /// @spec exploration/archive Hover control by state: Remove with no sessions commits without arm
    #[test]
    fn remove_with_no_sessions_commits_without_arm() {
        let mut exp = Exploration::new(1);
        exp.mark_archived();
        exp.session_count = 0;
        let id = exp.id.clone();

        assert!(matches!(
            exploration_hover_action(&exp, false),
            ExplorationHoverAction::Remove { armed: false, .. }
        ));

        let mut state = make_state(&id, &[]);
        state.explorations = vec![exp];
        let project = make_project(&[], &[]);
        let mut tabs = tab_bar::TabState::default();
        let mut interactions = HashMap::new();
        let hl = crate::highlight::SyntaxHighlighter::new();

        update(
            &mut state,
            &mut tabs,
            &mut interactions,
            Message::RemoveExploration(id.clone()),
            &project,
            &hl,
            false,
            1200.0,
        );
        assert!(state.explorations.iter().all(|e| e.id != id));
    }

    // ── archive browse ──────────────────────────────────────────────────

    fn entry_ids(entries: &[ArchivedEntry<'_>]) -> Vec<String> {
        entries
            .iter()
            .map(|e| match e {
                ArchivedEntry::Change(c) => c.name.clone(),
                ArchivedEntry::Exploration(x) => x.id.clone(),
            })
            .collect()
    }

    /// @spec archive/browse Interleaved archived rows: Archived non–idea-owned explorations appear with archived changes
    #[test]
    fn archived_explorations_appear_with_archived_changes() {
        let project = make_project(&[], &["2026-07-01-01-done"]);
        let mut exp = Exploration::new(1);
        exp.mark_archived();
        let entries = archived_entries(&project.archived_changes, std::slice::from_ref(&exp));
        let ids = entry_ids(&entries);
        assert!(ids.iter().any(|id| id == "2026-07-01-01-done"));
        assert!(ids.iter().any(|id| id == &exp.id));
    }

    /// @spec archive/browse Interleaved archived rows: Mixed archive rows order by archive date descending
    #[test]
    fn mixed_archive_rows_order_by_date_descending() {
        let project = make_project(
            &[],
            &["2026-01-01-01-old", "2026-07-12-09-new"],
        );
        let mut older_exp = Exploration::new(1);
        older_exp.archived_at = Some("2026-03-15T10:00:00+00:00".into());
        let mut newer_exp = Exploration::new(2);
        newer_exp.archived_at = Some("2026-07-12T18:00:00+00:00".into());
        let exps = vec![older_exp.clone(), newer_exp.clone()];
        let entries = archived_entries(&project.archived_changes, &exps);
        let ids = entry_ids(&entries);
        // newest first: late-day exploration, then 09 change, then March exp, then Jan change
        assert_eq!(
            ids,
            vec![
                newer_exp.id,
                "2026-07-12-09-new".to_string(),
                older_exp.id,
                "2026-01-01-01-old".to_string(),
            ]
        );
    }

    /// @spec archive/browse Interleaved archived rows: Idea-owned archived explorations stay off Change and Dashboard archived lists
    #[test]
    fn idea_owned_archived_explorations_stay_off_archived_lists() {
        let project = make_project(&[], &["2026-07-01-01-done"]);
        let mut exp = Exploration::new(1);
        exp.idea_path = Some("/ideas/x.md".into());
        exp.mark_archived();
        let entries = archived_entries(&project.archived_changes, std::slice::from_ref(&exp));
        assert!(!entries
            .iter()
            .any(|e| matches!(e, ArchivedEntry::Exploration(_))));
        assert_eq!(entries.len(), 1);
    }

    /// @spec archive/browse Archived section visibility: Archived section is empty only when both kinds are empty
    #[test]
    fn archived_section_present_with_only_exploration() {
        let project = make_project(&[], &[]);
        let mut exp = Exploration::new(1);
        exp.mark_archived();
        assert!(has_archived_section(
            &project.archived_changes,
            std::slice::from_ref(&exp)
        ));
        let entries = archived_entries(&project.archived_changes, std::slice::from_ref(&exp));
        assert_eq!(entries.len(), 1);
        assert!(matches!(entries[0], ArchivedEntry::Exploration(_)));
    }

    /// @spec archive/browse Archived section visibility: Change Archived section starts collapsed
    #[test]
    fn change_archived_section_starts_collapsed() {
        let state = State::new(None);
        assert!(!state.expanded_sections.contains("archived"));
        // Section still has rows to show when archives exist; collapsed by default.
        let project = make_project(&[], &["2026-07-01-01-done"]);
        assert!(has_archived_section(&project.archived_changes, &[]));
    }

    #[test]
    fn section_counts_match_list_membership() {
        let project = make_project(&["active-a", "active-b"], &["2026-07-01-01-done"]);
        let live = Exploration::new(1);
        let mut archived_exp = Exploration::new(2);
        archived_exp.mark_archived();
        let mut idea_owned = Exploration::new(3);
        idea_owned.idea_path = Some("/ideas/x.md".into());
        let mut idea_archived = Exploration::new(4);
        idea_archived.idea_path = Some("/ideas/y.md".into());
        idea_archived.mark_archived();
        let exps = vec![live, archived_exp, idea_owned, idea_archived];

        // Change: one live non–idea-owned exploration + two active changes.
        assert_eq!(
            change_section_count(&exps, &project.active_changes),
            1 + 2
        );
        // Archived: one change + one non–idea-owned archived exploration.
        assert_eq!(
            archived_section_count(&project.archived_changes, &exps),
            2
        );
        // Only-exploration archive still counts.
        assert_eq!(
            archived_section_count(&[], std::slice::from_ref(&exps[1])),
            1
        );
    }
}

#[cfg(test)]
mod explorer_tests {
    use super::*;

    fn paths(list: &[&str]) -> Vec<PathBuf> {
        list.iter().map(PathBuf::from).collect()
    }

    fn make_state() -> State {
        State::new(None)
    }

    #[test]
    fn tree_puts_sorted_dirs_before_sorted_files() {
        let mut dirs = HashSet::new();
        let tree = build_explorer_tree(
            &paths(&["zeta.rs", "src/b.rs", "src/a.rs", "Cargo.toml"]),
            &mut dirs,
        );
        let labels: Vec<&str> = tree.iter().map(|n| n.label.as_str()).collect();
        assert_eq!(labels, vec!["src", "Cargo.toml", "zeta.rs"]);
        let src_children: Vec<&str> = tree[0].children.iter().map(|n| n.id.as_str()).collect();
        assert_eq!(src_children, vec!["file:src/a.rs", "file:src/b.rs"]);
    }

    #[test]
    fn tree_ids_use_file_prefix_for_leaves_and_rel_paths_for_dirs() {
        let mut dirs = HashSet::new();
        let tree = build_explorer_tree(&paths(&["a/b/c.rs"]), &mut dirs);
        assert_eq!(tree[0].id, "a");
        assert_eq!(tree[0].children[0].id, "a/b");
        assert_eq!(tree[0].children[0].children[0].id, "file:a/b/c.rs");
        assert!(dirs.contains("a"));
        assert!(dirs.contains("a/b"));
    }

    #[test]
    fn set_project_files_prunes_stale_expanded_dirs() {
        let mut state = make_state();
        state.set_project_files(&paths(&["src/a.rs", "docs/b.md"]));
        state.expanded_explorer_dirs.insert("src".into());
        state.expanded_explorer_dirs.insert("docs".into());
        state.set_project_files(&paths(&["src/a.rs"]));
        assert!(state.expanded_explorer_dirs.contains("src"));
        assert!(!state.expanded_explorer_dirs.contains("docs"));
    }

    #[test]
    fn expand_ancestors_inserts_each_prefix() {
        let mut state = make_state();
        state.expand_explorer_ancestors("crates/duckboard/src/main.rs");
        assert!(state.expanded_explorer_dirs.contains("crates"));
        assert!(state.expanded_explorer_dirs.contains("crates/duckboard"));
        assert!(state.expanded_explorer_dirs.contains("crates/duckboard/src"));
        assert_eq!(state.expanded_explorer_dirs.len(), 3);
    }

    #[test]
    fn expand_ancestors_of_root_file_is_noop() {
        let mut state = make_state();
        state.expand_explorer_ancestors("Cargo.toml");
        assert!(state.expanded_explorer_dirs.is_empty());
    }

    #[test]
    fn vcs_tints_color_files_and_aggregate_dirs() {
        let cf = |path: &str, status: FileStatus| ChangedFile {
            path: PathBuf::from(path),
            status,
        };
        let tints = explorer_vcs_tints(&[
            cf("src/a.rs", FileStatus::Modified),
            cf("src/new/b.rs", FileStatus::Added),
            cf("docs/gone.md", FileStatus::Deleted),
        ]);

        let modified = theme::vcs_status_color(&FileStatus::Modified);
        let added = theme::vcs_status_color(&FileStatus::Added);

        assert_eq!(tints.get("file:src/a.rs"), Some(&modified));
        assert_eq!(tints.get("file:src/new/b.rs"), Some(&added));
        // Deleted files have no explorer row, but their dir is tinted with
        // the deletion color so the change is still discoverable.
        assert!(!tints.contains_key("file:docs/gone.md"));
        assert_eq!(
            tints.get("docs"),
            Some(&theme::vcs_status_color(&FileStatus::Deleted))
        );
        // Uniform dir takes its status color; mixed falls back to modified.
        assert_eq!(tints.get("src/new"), Some(&added));
        assert_eq!(tints.get("src"), Some(&modified));
    }

    #[test]
    fn flat_position_counts_only_visible_rows() {
        let mut state = make_state();
        state.set_project_files(&paths(&["src/a.rs", "src/b.rs", "Cargo.toml"]));
        // Collapsed: rows are [src, Cargo.toml].
        assert_eq!(
            state.explorer_flat_position("file:Cargo.toml"),
            Some((1, 2))
        );
        assert_eq!(state.explorer_flat_position("file:src/a.rs"), None);
        // Expanded: rows are [src, src/a.rs, src/b.rs, Cargo.toml].
        state.expanded_explorer_dirs.insert("src".into());
        assert_eq!(state.explorer_flat_position("file:src/a.rs"), Some((1, 4)));
        assert_eq!(
            state.explorer_flat_position("file:Cargo.toml"),
            Some((3, 4))
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
        assert_eq!(rows.len(), 3);
        match &rows[1] {
            FileTreeRow::File { depth, .. } => assert_eq!(*depth, 1),
            _ => panic!("expected file row"),
        }
    }
}
