//! duckboard — GUI for the duckspec framework, built with Iced 0.14.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use iced::event;
use iced::keyboard;
use iced::widget::{Space, column, container, row, stack};
use iced::{Element, Event, Length, Subscription, Task};

mod agent;
mod area;
mod chat_store;
pub mod config;
mod data;
pub mod highlight;
mod idea_store;
mod path_env;
mod scope;
mod theme;
mod title_hints;
mod vcs;
mod watcher;
mod widget;

use area::Area;
use area::interaction::{self, ActiveTab};
use data::ProjectData;
use widget::tab_bar;

// ── Constants for routing keys ──────────────────────────────────────────────

const KEY_CAPS: &str = "caps";
const KEY_CODEX: &str = "codex";

// ── State ────────────────────────────────────────────────────────────────────

struct State {
    active_area: Area,
    project: ProjectData,
    config: config::Config,
    dashboard: area::dashboard::State,
    ideas: area::ideas::State,
    change: area::change::State,
    caps: area::caps::State,
    codex: area::codex::State,
    settings: area::settings::State,
    file_finder: widget::file_finder::FileFinderState,
    text_search: widget::text_search::TextSearchState,
    project_picker: widget::project_picker::ProjectPickerState,
    quick_idea: widget::quick_idea::QuickIdeaState,
    /// Shared via `Arc` so background tasks (e.g. search-stack highlighting)
    /// can hold a handle without blocking the UI on syntax-set ownership.
    highlighter: Arc<highlight::SyntaxHighlighter>,
    /// Single tab stack shared across Change/Caps/Codex/Ideas. The active
    /// area drives the `preview` (pinned) slot via its list selection;
    /// `file_tabs` (closable) persist across area switches.
    tabs: tab_bar::TabState,
    /// Cached pinned tab per area, swapped in/out of `tabs.preview` on area
    /// switch so editor cursor + dirty state survive the round-trip.
    cached_previews: HashMap<Area, Option<tab_bar::Tab>>,
    /// Cached active-tab pointer per area. On `switch_area` we snapshot the
    /// outgoing area's `tabs.active` and restore the incoming area's, so a
    /// user looking at a file tab in one area returns to the same file tab
    /// (rather than always landing back on the preview).
    cached_active: HashMap<Area, tab_bar::ActiveTab>,
    /// Single interaction registry keyed by scope. One entry per
    /// (Caps | Codex | Change(name) | Exploration(id)). Survives area
    /// switches; the visible column reads from the active area's scope.
    interactions: HashMap<scope::Scope, interaction::InteractionState>,
}

impl State {
    fn new() -> Self {
        // Start with no project open — the user picks one from the dashboard
        // (button, recents list, or Cmd+O). Previously we walked up from CWD
        // for a `duckspec/` dir, but that breaks for launches from a GUI
        // (`.app` bundles have CWD=`/`) and surprised users who wanted a
        // blank slate.
        let project = ProjectData::default();
        let change = area::change::State::new(None);
        let caps_state = area::caps::State::default();

        let config = config::load();
        theme::set_fonts(&config);
        tracing::info!(
            recent = config.projects.recent.len(),
            "duckboard started with no project"
        );
        let mut interactions = HashMap::new();
        interactions.insert(scope::Scope::Caps, interaction::InteractionState::default());
        interactions.insert(scope::Scope::Codex, interaction::InteractionState::default());
        Self {
            active_area: Area::Dashboard,
            project,
            config,
            dashboard: area::dashboard::State::default(),
            ideas: area::ideas::State::default(),
            change,
            caps: caps_state,
            codex: area::codex::State::default(),
            settings: area::settings::State::default(),
            file_finder: widget::file_finder::FileFinderState::default(),
            text_search: widget::text_search::TextSearchState::default(),
            project_picker: widget::project_picker::ProjectPickerState::default(),
            quick_idea: widget::quick_idea::QuickIdeaState::default(),
            highlighter: Arc::new(highlight::SyntaxHighlighter::new()),
            tabs: tab_bar::TabState::default(),
            cached_previews: HashMap::new(),
            cached_active: HashMap::new(),
            interactions,
        }
    }

    /// Switch to the project rooted at `path`. Rebuilds subordinate area
    /// state so stale tabs / interactions from the previous project are
    /// discarded, then refreshes audit and recents.
    fn open_project(&mut self, path: PathBuf) {
        tracing::info!(path = %path.display(), "opening project");
        self.project = ProjectData::open(&path);
        // Rebuild area states tied to the old project root. Dropping the
        // previous `change::State` also drops any live interactions /
        // agent sessions / terminals from that project.
        self.change = area::change::State::new(self.project.project_root.as_deref());
        if let Some(root) = &self.project.project_root {
            self.change.set_changed_files(vcs::changed_files(root));
        }
        let mut caps_expanded = std::collections::HashSet::new();
        data::TreeNode::collect_parent_ids(&self.project.cap_tree, &mut caps_expanded);
        self.caps = area::caps::State {
            expanded_nodes: caps_expanded,
            ..Default::default()
        };
        self.codex = area::codex::State::default();
        self.ideas = area::ideas::State::for_project(self.project.project_root.as_deref());
        idea_store::reconcile(&mut self.ideas.ideas, &self.project);
        // Drop interactions / tabs from the prior project; reseed singletons.
        self.tabs = tab_bar::TabState::default();
        self.cached_previews.clear();
        self.cached_active.clear();
        self.interactions.clear();
        self.interactions
            .insert(scope::Scope::Caps, interaction::InteractionState::default());
        self.interactions
            .insert(scope::Scope::Codex, interaction::InteractionState::default());
        self.project.revalidate();
        self.active_area = Area::Dashboard;

        self.config.projects.touch(&path);
        if let Err(e) = config::save(&self.config) {
            tracing::warn!("failed to persist recent projects: {e}");
        }
    }

    /// Resolve a scope key (bare change name / exploration id / "caps" / "codex")
    /// to its interaction state. Routes via the active area when the scope key
    /// alone is ambiguous between Change(name) and Exploration(id).
    fn interaction_mut(&mut self, scope: &str) -> Option<&mut interaction::InteractionState> {
        let key = self.scope_for_key(scope);
        self.interactions.get_mut(&key)
    }

    /// Build the appropriate `Scope` for a raw scope key, classifying via
    /// the change area's exploration list when the key is not a singleton.
    fn scope_for_key(&self, scope: &str) -> scope::Scope {
        match scope {
            KEY_CAPS => scope::Scope::Caps,
            KEY_CODEX => scope::Scope::Codex,
            _ => self.change.scope_for(scope),
        }
    }

    /// Resolve a stable `InteractionState::instance_id` to its state.
    fn interaction_mut_by_ix_id(
        &mut self,
        ix_id: u64,
    ) -> Option<&mut interaction::InteractionState> {
        self.interactions
            .values_mut()
            .find(|ix| ix.instance_id == ix_id)
    }

    /// Resolve a composite routing key `<instance_id>/<session_id>` to the session bundle.
    fn agent_session_mut(&mut self, key: &str) -> Option<&mut interaction::AgentSession> {
        let (ix_id_str, session_id) = key.split_once('/')?;
        let ix_id: u64 = ix_id_str.parse().ok()?;
        let ix = self.interaction_mut_by_ix_id(ix_id)?;
        ix.find_session_mut(session_id)
    }

    /// Compute the active scope from `active_area` and that area's selection.
    fn active_scope(&self) -> Option<scope::Scope> {
        match self.active_area {
            Area::Caps => Some(scope::Scope::Caps),
            Area::Codex => Some(scope::Scope::Codex),
            Area::Change => {
                let name = self.change.selected_change.as_deref()?;
                Some(self.change.scope_for(name))
            }
            Area::Ideas => {
                let path = self.ideas.selected.as_deref()?;
                self.ideas.scope_for_path(path)
            }
            Area::Dashboard | Area::Settings => None,
        }
    }

    /// Active area's interaction (read-only) plus the scope-key string used by
    /// title-hint refreshers etc.
    fn active_interaction(&self) -> Option<(&interaction::InteractionState, String)> {
        let scope = self.active_scope()?;
        let key = scope.key().to_string();
        let ix = self.interactions.get(&scope)?;
        Some((ix, key))
    }

    /// Active scope's key as a `String`, when one exists.
    fn active_interaction_key(&self) -> Option<String> {
        Some(self.active_scope()?.key().to_string())
    }
}

// ── Messages ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum Message {
    AreaSelected(Area),
    Refresh,
    Dashboard(area::dashboard::Message),
    Ideas(area::ideas::Message),
    Change(area::change::Message),
    Caps(area::caps::Message),
    Codex(area::codex::Message),
    /// Shared content column tab interactions (select / close / editor /
    /// search-slice / open-in-tab). Wraps the tab_bar widget's child message
    /// kinds since the content column lives at the top level after the
    /// shared-tabs refactor.
    TabSelect(usize),
    TabClose(usize),
    TabContent(tab_bar::TabContentMsg),
    /// Shared interaction-column messages (sessions, terminals, agent input).
    /// Routed to `state.interactions[active_scope]`.
    Interaction(interaction::Msg),
    // File finder
    FileFinder(widget::file_finder::Msg),
    // Project-wide text search
    TextSearch(widget::text_search::Msg),
    // Project picker (choose a project root to open).
    ProjectPicker(widget::project_picker::Msg),
    // Quick idea capture/jump modal (cmd-i).
    QuickIdea(widget::quick_idea::Msg),
    /// Open a project rooted at this path (from picker confirm or recents).
    OpenProject(PathBuf),
    // Async search-stack highlighting: one message per unique file once the
    // background `highlight_lines_until` job finishes. `spans` is wrapped in
    // `Arc` so the message is cheap to clone; the handler clones the inner
    // `Vec` into each slice sharing `abs_path`.
    SearchStackHighlighted {
        tab_id: String,
        abs_path: std::path::PathBuf,
        spans: Arc<Vec<Vec<highlight::HighlightSpan>>>,
    },
    // Async file-tab highlighting. `version` is the `EditorState`'s
    // `highlight_version` at spawn time; the handler drops stale spans
    // whose version no longer matches (i.e. the user edited during the
    // highlight window).
    FileTabHighlighted {
        area: Area,
        tab_id: String,
        version: u64,
        spans: Arc<Vec<Vec<highlight::HighlightSpan>>>,
    },
    // Async diff-tab highlighting. Carries the computed syntect spans for
    // both sides of the diff; the handler rebuilds the editor's composite
    // per-line spans via `diff_view::build_diff_spans`.
    DiffTabHighlighted {
        area: Area,
        tab_id: String,
        version: u64,
        highlight: Arc<widget::diff_view::DiffHighlight>,
    },
    // File watcher
    FileChanged(Vec<watcher::FileEvent>),
    // Keyboard
    KeyPress(keyboard::Key, keyboard::Modifiers, Option<String>),
    // Per-terminal PTY events. `ix_id` is the stable `InteractionState::instance_id`,
    // `terminal_id` identifies the specific terminal tab within that interaction.
    PtyEvent(u64, u64, widget::terminal::PtyEvent),
    // Clipboard → PTY paste (scope name identifies the interaction).
    TerminalPaste(String, Option<String>),
    // Per-instance agent events. Key format: `<instance_id>/<session_id>`.
    AgentEvent(String, agent::AgentEvent),
    // Result of the one-shot title-summary call kicked off after the first
    // successful turn of a fresh session. Key matches AgentEvent routing.
    SessionTitleReady {
        key: String,
        result: Result<String, String>,
    },
    // Settings
    Settings(area::settings::Message),
    // System theme changed
    ThemeChanged(theme::ColorMode),
    // Animation tick for the streaming indicator; only fires while a session
    // is streaming (see `subscription`).
    StreamTick,
}

// ── Update ───────────────────────────────────────────────────────────────────

fn update(state: &mut State, message: Message) -> Task<Message> {
    // The inline tag editor in the Ideas pinned toolbar dismisses itself
    // when the user clicks anywhere that would naturally pull focus from
    // the text_input — clicking the editor body, switching areas, or
    // running an idea-level action. Tag-related messages (chip clicks, +
    // Tag, the input's own keystrokes) keep it alive; everything else in
    // the explicit list below clears it before the action proceeds.
    if state.ideas.tag_input.is_some() && tag_input_loses_focus_on(&message) {
        state.ideas.tag_input = None;
        state.ideas.tag_input_editing = None;
    }
    match message {
        Message::AreaSelected(area) => {
            switch_area(state, area);
            if area == Area::Settings {
                area::settings::update(
                    &mut state.settings,
                    &mut state.config,
                    area::settings::Message::LoadFonts,
                );
            }
        }
        Message::Refresh => {
            reload_and_reconcile(state);
            let mut tasks: Vec<Task<Message>> = Vec::new();
            refresh_open_tabs(state, &mut tasks);
            refresh_changed_files(state);
            state.project.revalidate();
            tracing::info!("project reloaded");
            return Task::batch(tasks);
        }
        Message::FileFinder(msg) => {
            use widget::file_finder::Msg;
            match msg {
                Msg::Open => {
                    if let Some(root) = &state.project.project_root {
                        state.file_finder.open(root);
                        for ix in state.interactions.values_mut() {
                            ix.terminal_focused = false;
                        }
                        return iced::widget::operation::focus("file-finder-input");
                    }
                }
                Msg::Close => {
                    state.file_finder.close();
                }
                Msg::QueryChanged(q) => {
                    state.file_finder.set_query(q);
                }
                Msg::SelectNext => {
                    state.file_finder.select_next();
                }
                Msg::SelectPrev => {
                    state.file_finder.select_prev();
                }
                Msg::Confirm => {
                    let mut task = Task::none();
                    if let Some(rel_path) = state.file_finder.selected_path() {
                        if let Some(root) = &state.project.project_root {
                            let abs = root.join(&rel_path);
                            if let Ok(content) = std::fs::read_to_string(&abs) {
                                let id = format!("file:{}", rel_path.display());
                                let title = rel_path
                                    .file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_else(|| rel_path.display().to_string());
                                let area = match state.active_area {
                                    Area::Dashboard | Area::Settings => Area::Change,
                                    other => other,
                                };
                                switch_area(state, area);
                                state
                                    .tabs
                                    .open_file(id.clone(), title, content, Some(abs.clone()));
                                if let Some(tab) =
                                    state.tabs.file_tabs.iter_mut().find(|t| t.id == id)
                                    && let tab_bar::TabView::Editor { editor, .. } = &mut tab.view
                                {
                                    task = spawn_file_tab_highlight(
                                        area,
                                        id,
                                        editor,
                                        state.highlighter.clone(),
                                        false,
                                    );
                                }
                            }
                        }
                        state.file_finder.close();
                    }
                    return task;
                }
            }
        }
        Message::TextSearch(msg) => {
            use widget::text_search::Msg;
            match msg {
                Msg::Open => {
                    state.text_search.open();
                    for ix in state.interactions.values_mut() {
                        ix.terminal_focused = false;
                    }
                    return iced::widget::operation::focus(
                        widget::text_search::SEARCH_INPUT_ID,
                    );
                }
                Msg::Close => {
                    state.text_search.close();
                }
                Msg::QueryChanged(q) => {
                    state.text_search.query = q.clone();
                    state.text_search.selected = 0;
                    if q.is_empty() {
                        // Bump the id so any in-flight search's ResultsReady
                        // is discarded instead of repopulating the list.
                        state.text_search.latest_query_id += 1;
                        state.text_search.results.clear();
                        state.text_search.searching = false;
                        return Task::none();
                    }
                    return spawn_text_search(state, q);
                }
                Msg::ScopeSelected(scope) => {
                    state.text_search.scope = scope;
                    let q = state.text_search.query.clone();
                    let refocus = iced::widget::operation::focus(
                        widget::text_search::SEARCH_INPUT_ID,
                    );
                    if !q.is_empty() {
                        return Task::batch([spawn_text_search(state, q), refocus]);
                    }
                    return refocus;
                }
                Msg::SelectNext => {
                    state.text_search.select_next();
                }
                Msg::SelectPrev => {
                    state.text_search.select_prev();
                }
                Msg::ConfirmTop => {
                    let mut task = Task::none();
                    if let Some(hit) = state.text_search.selected_hit().cloned() {
                        let all = state.text_search.results.clone();
                        task = open_search_hit_as_file(state, &hit, &all);
                    }
                    state.text_search.close();
                    return task;
                }
                Msg::ConfirmStack => {
                    let query = state.text_search.query.clone();
                    let hits: Vec<_> = state.text_search.results.clone();
                    state.text_search.close();
                    if !hits.is_empty() {
                        return open_search_stack_tab(state, &query, hits);
                    }
                }
                Msg::ResultsReady(query_id, results) => {
                    if query_id == state.text_search.latest_query_id {
                        state.text_search.results = results;
                        state.text_search.searching = false;
                        state.text_search.selected = 0;
                    }
                    // Stale results: discard silently.
                }
            }
        }
        Message::ProjectPicker(msg) => {
            use widget::project_picker::Msg;
            match msg {
                Msg::Open => {
                    state.project_picker.open();
                    for ix in state.interactions.values_mut() {
                        ix.terminal_focused = false;
                    }
                    return Task::batch([
                        iced::widget::operation::focus(widget::project_picker::INPUT_ID),
                        iced::widget::operation::move_cursor_to_end(
                            widget::project_picker::INPUT_ID,
                        ),
                    ]);
                }
                Msg::Close => {
                    state.project_picker.close();
                }
                Msg::QueryChanged(q) => {
                    if state.project_picker.handle_input(q) {
                        // The handler rewrote the query (erased a full
                        // segment); snap the cursor to the new end so the
                        // widget's internal offset doesn't land past-EOL.
                        return iced::widget::operation::move_cursor_to_end(
                            widget::project_picker::INPUT_ID,
                        );
                    }
                }
                Msg::SelectNext => {
                    state.project_picker.select_next();
                }
                Msg::SelectPrev => {
                    state.project_picker.select_prev();
                }
                Msg::TabComplete => {
                    state.project_picker.tab_complete();
                    // Snap the cursor to the end of the freshly-expanded
                    // path so the next keystroke continues typing instead
                    // of inserting mid-word.
                    return iced::widget::operation::move_cursor_to_end(
                        widget::project_picker::INPUT_ID,
                    );
                }
                Msg::Confirm => {
                    if let Some(path) = state.project_picker.resolved_path() {
                        state.project_picker.close();
                        return update(state, Message::OpenProject(path));
                    }
                }
                Msg::PickPath(path) => {
                    state.project_picker.close();
                    return update(state, Message::OpenProject(path));
                }
            }
        }
        Message::OpenProject(path) => {
            state.open_project(path);
        }
        Message::QuickIdea(msg) => {
            use widget::quick_idea::Msg;
            match msg {
                Msg::Open => {
                    if state.project.project_root.is_none() {
                        return Task::none();
                    }
                    state
                        .quick_idea
                        .open(&state.ideas.ideas, state.highlighter.as_ref());
                    for ix in state.interactions.values_mut() {
                        ix.terminal_focused = false;
                    }
                    return iced::widget::operation::focus(widget::quick_idea::INPUT_ID);
                }
                Msg::Close => {
                    state.quick_idea.close();
                }
                Msg::EditorAction(action) => {
                    state
                        .quick_idea
                        .apply_editor_action(action, state.highlighter.as_ref());
                }
                Msg::Submit => {
                    if state.quick_idea.selected.is_some() {
                        state
                            .quick_idea
                            .load_selected(state.highlighter.as_ref());
                        return iced::widget::operation::focus(widget::quick_idea::INPUT_ID);
                    }
                    let payload = widget::quick_idea::build_save_payload(&state.quick_idea);
                    if !widget::quick_idea::body_is_savable(&payload.body) {
                        return Task::none();
                    }
                    let project_root = state.project.project_root.clone();
                    save_quick_idea(state, payload, project_root.as_deref());
                    state.quick_idea.close();
                }
                Msg::SelectNext => {
                    state.quick_idea.select_next();
                }
                Msg::SelectPrev => {
                    state.quick_idea.select_prev();
                }
                Msg::OpenTagInput => {
                    state.quick_idea.open_tag_input();
                    return iced::widget::operation::focus(widget::quick_idea::TAG_INPUT_ID);
                }
                Msg::CancelTagInput => {
                    state.quick_idea.cancel_tag_input();
                    return iced::widget::operation::focus(widget::quick_idea::INPUT_ID);
                }
                Msg::TagInputChanged(s) => {
                    state.quick_idea.set_tag_input(s);
                }
                Msg::SubmitTagInput => {
                    state.quick_idea.submit_tag_input();
                    return iced::widget::operation::focus(widget::quick_idea::INPUT_ID);
                }
                Msg::RemoveTag(idx) => {
                    state.quick_idea.remove_tag(idx);
                }
                Msg::ChipClick(idx) => {
                    if widget::terminal::current_modifiers().shift() {
                        state.quick_idea.promote_tag(idx);
                    } else {
                        state.quick_idea.edit_tag(idx);
                        return iced::widget::operation::focus(widget::quick_idea::TAG_INPUT_ID);
                    }
                }
            }
        }
        Message::SearchStackHighlighted {
            tab_id,
            abs_path,
            spans,
        } => {
            if let Some(tab) = state.tabs.file_tabs.iter_mut().find(|t| t.id == tab_id)
                && let tab_bar::TabView::SearchStack { slices, .. } = &mut tab.view
            {
                for slice in slices.iter_mut() {
                    if slice.abs_path == abs_path {
                        slice.editor.highlight_spans = Some((*spans).clone());
                    }
                }
            }
        }
        Message::FileTabHighlighted {
            area,
            tab_id,
            version,
            spans,
        } => {
            let _ = area;
            if let Some(editor) = find_editor_mut(&mut state.tabs, &tab_id)
                && editor.highlight_version == version
            {
                editor.highlight_spans = Some((*spans).clone());
            }
        }
        Message::DiffTabHighlighted {
            area,
            tab_id,
            version,
            highlight,
        } => {
            let _ = area;
            if let Some((editor, diff_data)) = find_diff_tab_mut(&mut state.tabs, &tab_id)
                && editor.highlight_version == version
            {
                editor.highlight_spans = Some(widget::diff_view::build_diff_spans(
                    &diff_data,
                    Some(&highlight),
                ));
            }
        }
        Message::FileChanged(events) => {
            tracing::debug!(count = events.len(), "file watcher events received");
            let duckspec_root = state.project.duckspec_root.clone();
            let project_root = state.project.project_root.clone();
            let mut tree_changed = false;
            let mut vcs_state_changed = false;
            let mut highlight_tasks: Vec<Task<Message>> = Vec::new();

            for event in &events {
                match event {
                    watcher::FileEvent::Modified(path) => {
                        if let Some(root) = duckspec_root.as_deref() {
                            if let Ok(rel) = path.strip_prefix(root) {
                                let id = rel.to_string_lossy().to_string();
                                if let Some(content) = state.project.read_artifact(&id) {
                                    refresh_artifact_tabs(
                                        state,
                                        &id,
                                        content,
                                        &mut highlight_tasks,
                                    );
                                }
                            }
                            if path.starts_with(root) {
                                tree_changed = true;
                            }
                        }
                        if let Some(root) = project_root.as_deref() {
                            refresh_file_tabs_for_path(
                                state,
                                root,
                                path,
                                &mut highlight_tasks,
                            );
                            refresh_diff_tabs_for_path(
                                state,
                                root,
                                path,
                                &mut highlight_tasks,
                            );
                        }
                    }
                    watcher::FileEvent::Removed(path) => {
                        if let Some(root) = duckspec_root.as_deref() {
                            if let Ok(rel) = path.strip_prefix(root) {
                                let id = rel.to_string_lossy().to_string();
                                state.tabs.close_by_id(&id);
                                close_cached_tabs(state, &id);
                            }
                            if path.starts_with(root) {
                                tree_changed = true;
                            }
                        }
                        if let Some(root) = project_root.as_deref()
                            && let Ok(rel) = path.strip_prefix(root)
                        {
                            let diff_id = format!("vcs:{}", rel.display());
                            state.tabs.close_by_id(&diff_id);
                            close_cached_tabs(state, &diff_id);
                        }
                    }
                    watcher::FileEvent::VcsStateChanged(path) => {
                        tracing::debug!(path = %path.display(), "git state changed — refreshing");
                        vcs_state_changed = true;
                    }
                }
            }

            if tree_changed && reload_and_reconcile(state) {
                // Tab IDs were rewritten to new archive paths; re-read
                // their content from disk so editors reflect the moved files.
                refresh_open_tabs(state, &mut highlight_tasks);
            }

            if vcs_state_changed && let Some(root) = project_root.as_deref() {
                refresh_all_diff_tabs(state, root, &mut highlight_tasks);
            }

            refresh_changed_files(state);

            return Task::batch(highlight_tasks);
        }
        Message::Dashboard(msg) => {
            match &msg {
                area::dashboard::Message::OpenProjectPicker => {
                    return update(
                        state,
                        Message::ProjectPicker(widget::project_picker::Msg::Open),
                    );
                }
                area::dashboard::Message::OpenRecent(path) => {
                    return update(state, Message::OpenProject(path.clone()));
                }
                area::dashboard::Message::ChangeClicked(name)
                | area::dashboard::Message::ArchivedChangeClicked(name)
                | area::dashboard::Message::ExplorationClicked(name) => {
                    switch_area(state, Area::Change);
                    area::change::update(
                        &mut state.change,
                        &mut state.tabs,
                        &mut state.interactions,
                        area::change::Message::SelectChange(name.clone()),
                        &state.project,
                        &state.highlighter,
                    );
                }
                area::dashboard::Message::AddExploration => {
                    switch_area(state, Area::Change);
                    area::change::update(
                        &mut state.change,
                        &mut state.tabs,
                        &mut state.interactions,
                        area::change::Message::AddExploration,
                        &state.project,
                        &state.highlighter,
                    );
                }
                area::dashboard::Message::SelectAuditError {
                    change,
                    artifact_id,
                } => {
                    switch_area(state, Area::Change);
                    area::change::update(
                        &mut state.change,
                        &mut state.tabs,
                        &mut state.interactions,
                        area::change::Message::OpenArtifact {
                            change: change.clone(),
                            artifact_id: artifact_id.clone(),
                        },
                        &state.project,
                        &state.highlighter,
                    );
                }
            }
            area::dashboard::update(&mut state.dashboard, msg);
        }
        Message::Change(msg) => {
            match msg {
                area::change::Message::SelectChangedFile(path) => {
                    return open_diff_preview(state, Area::Change, &path);
                }
                area::change::Message::OpenIdeaForChange(change_name) => {
                    if let Some(idea_path) = state.ideas.idea_path_for_change(&change_name) {
                        switch_area(state, Area::Ideas);
                        area::ideas::update(
                            &mut state.ideas,
                            &mut state.tabs,
                            &mut state.interactions,
                            area::ideas::Message::SelectIdea(idea_path),
                            &state.project,
                            &state.highlighter,
                        );
                    }
                }
                msg => {
                    let needs_focus =
                        is_chat_focus_msg(extract_change_interaction_msg(&msg));
                    area::change::update(
                        &mut state.change,
                        &mut state.tabs,
                        &mut state.interactions,
                        msg,
                        &state.project,
                        &state.highlighter,
                    );
                    if needs_focus {
                        return focus_chat_input();
                    }
                }
            }
        }
        Message::Caps(msg) => {
            let needs_focus = is_chat_focus_msg(extract_caps_interaction_msg(&msg));
            let ix = state
                .interactions
                .entry(scope::Scope::Caps)
                .or_default();
            area::caps::update(
                &mut state.caps,
                &mut state.tabs,
                ix,
                msg,
                &state.project,
                &state.highlighter,
            );
            if needs_focus {
                return focus_chat_input();
            }
        }
        Message::Codex(msg) => {
            let needs_focus = is_chat_focus_msg(extract_codex_interaction_msg(&msg));
            let ix = state
                .interactions
                .entry(scope::Scope::Codex)
                .or_default();
            area::codex::update(
                &mut state.codex,
                &mut state.tabs,
                ix,
                msg,
                &state.project,
                &state.highlighter,
            );
            if needs_focus {
                return focus_chat_input();
            }
        }
        Message::Ideas(msg) => {
            // Hard delete cascades to the attached exploration (if any). Run
            // the cascade BEFORE ideas::update so we can still look up the
            // idea's exploration id from frontmatter.
            if let area::ideas::Message::DeleteIdea(ref path) = msg {
                let exp_id = state
                    .ideas
                    .ideas
                    .iter()
                    .find(|i| &i.abs_path == path)
                    .and_then(|i| i.frontmatter.exploration.clone());
                if let Some(exp_id) = exp_id {
                    state.change.explorations.retain(|e| e.id != exp_id);
                    state
                        .interactions
                        .remove(&scope::Scope::Exploration(exp_id.clone()));
                    if state.change.selected_change.as_deref() == Some(&exp_id) {
                        state.change.selected_change = None;
                    }
                    chat_store::delete_scope(&exp_id, state.project.project_root.as_deref());
                    chat_store::save_explorations(
                        &state.change.explorations,
                        state.change.exploration_counter,
                        state.project.project_root.as_deref(),
                    );
                }
            }
            if let area::ideas::Message::StartExploration(ref path) = msg {
                state.change.exploration_counter += 1;
                let mut exp = chat_store::Exploration::new(state.change.exploration_counter);
                let exp_id = exp.id.clone();
                let old_path = path.clone();
                let mut new_path = old_path.clone();
                if let Some(idea) =
                    state.ideas.ideas.iter_mut().find(|i| i.abs_path == old_path)
                {
                    idea.frontmatter.exploration = Some(exp_id.clone());
                    idea.state = idea_store::IdeaState::Exploration;
                    let body = idea_store::read_body(&idea.abs_path).unwrap_or_default();
                    if let Err(e) = idea_store::save_idea(
                        idea,
                        &body,
                        state.project.project_root.as_deref(),
                    ) {
                        tracing::warn!("failed to save idea on Explore: {e}");
                    }
                    new_path = idea.abs_path.clone();
                }
                exp.idea_path = Some(new_path.display().to_string());
                state.change.explorations.push(exp);
                chat_store::save_explorations(
                    &state.change.explorations,
                    state.change.exploration_counter,
                    state.project.project_root.as_deref(),
                );
                area::ideas::update(
                    &mut state.ideas,
                    &mut state.tabs,
                    &mut state.interactions,
                    area::ideas::Message::SelectIdea(new_path),
                    &state.project,
                    &state.highlighter,
                );
                return focus_chat_input();
            }
            if let area::ideas::Message::OpenChange(ref change_name) = msg {
                let change_name = change_name.clone();
                switch_area(state, Area::Change);
                area::change::update(
                    &mut state.change,
                    &mut state.tabs,
                    &mut state.interactions,
                    area::change::Message::SelectChange(change_name),
                    &state.project,
                    &state.highlighter,
                );
                return Task::none();
            }
            // Chip click: shift held → promote to primary; otherwise open
            // the input pre-filled for rename. Modifier state lives in a
            // process-wide cell maintained by the global key event handler.
            if let area::ideas::Message::ChipClick(idx) = msg {
                let resolved = if widget::terminal::current_modifiers().shift() {
                    area::ideas::Message::PromoteTag(idx)
                } else {
                    area::ideas::Message::EditTag(idx)
                };
                return update(state, Message::Ideas(resolved));
            }
            let needs_focus = is_chat_focus_msg(extract_ideas_interaction_msg(&msg));
            let focus_tag_input = matches!(
                msg,
                area::ideas::Message::OpenTagInput | area::ideas::Message::EditTag(_)
            );
            area::ideas::update(
                &mut state.ideas,
                &mut state.tabs,
                &mut state.interactions,
                msg,
                &state.project,
                &state.highlighter,
            );
            if focus_tag_input {
                return iced::widget::operation::focus(area::ideas::TAG_INPUT_ID);
            }
            if needs_focus {
                return focus_chat_input();
            }
        }
        Message::Settings(msg) => {
            area::settings::update(&mut state.settings, &mut state.config, msg);
            theme::set_fonts(&state.config);
        }
        Message::TabSelect(idx) => state.tabs.select(idx),
        Message::TabClose(idx) => state.tabs.close(idx),
        Message::TabContent(tab_bar::TabContentMsg::EditorAction(action)) => {
            // Cmd-S on the ideas pinned tab routes through ideas::SaveBody so
            // frontmatter is rederived and the file moves on title/tag change.
            // The generic save path treats the editor `path` as the truth and
            // skips this tab (which has `path: None`).
            if matches!(action, widget::text_edit::EditorAction::SaveRequested)
                && state.active_area == Area::Ideas
                && state
                    .tabs
                    .active_tab()
                    .is_some_and(|t| t.id.starts_with(area::ideas::PINNED_TAB_PREFIX))
            {
                return update(state, Message::Ideas(area::ideas::Message::SaveBody));
            }
            return handle_editor_action(
                &mut state.tabs,
                state.active_area,
                action,
                state.highlighter.clone(),
            );
        }
        Message::TabContent(tab_bar::TabContentMsg::OpenInNewTab(rel_path)) => {
            // Only meaningful in Change area (diff tabs surface `OpenInNewTab`);
            // open the file as a new file tab and rehighlight inline.
            if let Some(root) = &state.project.project_root {
                let abs = root.join(&rel_path);
                if let Ok(content) = std::fs::read_to_string(&abs) {
                    let id = format!("file:{}", rel_path.display());
                    let title = rel_path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| rel_path.display().to_string());
                    state
                        .tabs
                        .open_file(id.clone(), title, content, Some(abs.clone()));
                    if let Some(tab) = state.tabs.file_tabs.iter_mut().find(|t| t.id == id)
                        && let tab_bar::TabView::Editor { editor, .. } = &mut tab.view
                    {
                        rehighlight(editor, &id, &state.highlighter);
                    }
                }
            }
        }
        Message::TabContent(tab_bar::TabContentMsg::SearchSliceAction(idx, action)) => {
            handle_search_slice_action(&mut state.tabs, idx, action);
        }
        Message::TabContent(tab_bar::TabContentMsg::OpenSearchSlice(idx)) => {
            handle_open_search_slice(&mut state.tabs, idx, &state.highlighter);
        }
        Message::Interaction(msg) => {
            // Route the interaction message to the active area's update fn so
            // its scope-specific handling (Change multi-session vs Caps single
            // vs Ideas pre/post-promotion) runs.
            return route_interaction(state, msg);
        }
        // Clipboard → PTY paste.
        Message::TerminalPaste(key, Some(text)) => {
            if let Some(ix) = state.interaction_mut(&key)
                && let Some(tt) = ix.active_terminal_mut()
            {
                tt.state.paste_text(&text);
            }
        }
        Message::TerminalPaste(_, None) => {}
        // Per-terminal PTY events
        Message::PtyEvent(ix_id, terminal_id, evt) => {
            use widget::terminal::PtyEvent;
            let Some(ix) = state.interaction_mut_by_ix_id(ix_id) else {
                return Task::none();
            };
            let Some(idx) = ix.find_terminal_index(terminal_id) else {
                return Task::none();
            };
            match evt {
                PtyEvent::Ready(writer, master) => {
                    if let Some(tt) = ix.terminals.get_mut(idx) {
                        tt.state.set_writer(writer.into_writer());
                        tt.state.set_master(master.into_master());
                        tracing::info!(ix_id, terminal_id, "PTY writer ready");
                    }
                }
                PtyEvent::Output(bytes) => {
                    if let Some(tt) = ix.terminals.get_mut(idx) {
                        tt.state.feed(&bytes);
                    }
                }
                PtyEvent::Exited => {
                    tracing::info!(ix_id, terminal_id, "PTY child exited");
                    ix.terminals.remove(idx);
                    ix.active_tab = interaction::adjust_active_after_remove(ix.active_tab, idx);
                    ix.terminal_focused =
                        ix.visible && matches!(ix.active_tab, ActiveTab::Terminal(_));
                }
            }
        }
        // Per-instance agent events — key is `<scope>/<session_id>`.
        Message::AgentEvent(key, evt) => {
            use agent::AgentEvent;
            let proj_root = state.project.project_root.clone();
            // `(working_dir, scope_key, scope_kind, first_user_msg, card_description)`.
            let mut title_task_input: Option<(
                PathBuf,
                String,
                scope::ScopeKind,
                String,
                Option<String>,
            )> = None;
            {
                let Some(ax) = state.agent_session_mut(&key) else {
                    return Task::none();
                };
                match evt {
                    AgentEvent::Ready(handle) => {
                        // Seed the worker with a previously-persisted Claude session
                        // id so the next prompt resumes that conversation.
                        if let Some(sid) = ax.session.claude_session_id.clone() {
                            handle.set_session_id(sid);
                        }
                        ax.agent_handle = Some(handle);
                        tracing::info!(key, "agent handle ready");
                    }
                    AgentEvent::CommandsAvailable(commands) => {
                        tracing::info!(key, count = commands.len(), "slash commands discovered");
                        ax.chat_commands = commands;
                    }
                    AgentEvent::ContentDelta { text } => {
                        ax.session.pending_text.push_str(&text);
                    }
                    AgentEvent::ToolUse { id, name, input } => {
                        flush_pending_text(&mut ax.session);
                        ax.session.messages.push(chat_store::ChatMessage {
                            role: chat_store::Role::Assistant,
                            content: vec![chat_store::ContentBlock::ToolUse { id, name, input }],
                            timestamp: String::new(),
                        });
                    }
                    AgentEvent::ToolResult { id, name, output } => {
                        ax.session.messages.push(chat_store::ChatMessage {
                            role: chat_store::Role::Assistant,
                            content: vec![chat_store::ContentBlock::ToolResult {
                                id,
                                name,
                                output,
                            }],
                            timestamp: String::new(),
                        });
                    }
                    AgentEvent::TurnComplete => {
                        flush_pending_text(&mut ax.session);
                        ax.session.is_streaming = false;
                        if let Err(e) = chat_store::save_session(&ax.session, proj_root.as_deref())
                        {
                            tracing::error!("failed to save chat session: {e}");
                        }
                        // Kick off a one-shot title summary after the first
                        // successful turn. Only for change / exploration
                        // scopes; caps and codex don't get summarised.
                        if ax.session.title.is_none()
                            && matches!(
                                ax.scope_kind,
                                scope::ScopeKind::Change | scope::ScopeKind::Exploration
                            )
                            && let Some(handle) = ax.agent_handle.as_ref()
                            && let Some(user) = chat_store::first_user_text(&ax.session)
                        {
                            title_task_input = Some((
                                handle.working_dir().to_path_buf(),
                                ax.session.scope.clone(),
                                ax.scope_kind,
                                user,
                                ax.card_description.clone(),
                            ));
                        }
                    }
                    AgentEvent::Error(msg) => {
                        tracing::error!(key, "agent error: {msg}");
                        ax.session.is_streaming = false;
                        ax.session.messages.push(chat_store::ChatMessage {
                            role: chat_store::Role::System,
                            content: vec![chat_store::ContentBlock::Text(format!("Error: {msg}"))],
                            timestamp: String::new(),
                        });
                    }
                    AgentEvent::SessionIdUpdated { session_id } => {
                        ax.session.claude_session_id = Some(session_id);
                    }
                    AgentEvent::UsageUpdate {
                        model,
                        input_tokens,
                        output_tokens,
                        context_window,
                    } => {
                        if let Some(m) = model {
                            ax.agent_model = m;
                        }
                        if input_tokens > 0 {
                            ax.agent_input_tokens = input_tokens;
                        }
                        if output_tokens > 0 {
                            ax.agent_output_tokens = output_tokens;
                        }
                        if let Some(cw) = context_window {
                            ax.agent_context_window = cw;
                        }
                    }
                    AgentEvent::ProcessExited => {
                        tracing::info!(key, "agent process exited");
                        ax.agent_handle = None;
                        ax.session.is_streaming = false;
                    }
                }
            }
            let State {
                interactions,
                highlighter,
                ..
            } = state;
            let ax = resolve_session_mut(interactions, &key);
            let mut should_snap_to_bottom = false;
            if let Some(ax) = ax {
                let is_streaming = ax.session.is_streaming;
                interaction::rebuild_chat_editor(ax, highlighter);
                should_snap_to_bottom = ax.stick_to_bottom;
                if !is_streaming {
                    ax.esc_count = 0;
                    // Auto-flush a queued message once the current turn is
                    // done (natural completion or user-triggered interrupt).
                    // Only flush if the agent is still attached — on
                    // ProcessExited the handle is gone and we'd lose the text.
                    if ax.agent_handle.is_some()
                        && let Some(q) = ax.queue_editor.take()
                    {
                        let text = q.text();
                        if !text.trim().is_empty() {
                            interaction::send_prompt_text(ax, text, highlighter);
                        }
                    }
                }
            }

            let snap_task = if should_snap_to_bottom {
                iced::widget::operation::snap_to_end(widget::agent_chat::CHAT_SCROLLABLE_ID)
            } else {
                Task::none()
            };

            if let Some((working_dir, scope_key, scope_kind, user, card_description)) =
                title_task_input
            {
                use duckchat::ContextHook;
                let mut hints = Vec::new();
                if let Some(hint) = title_hints::build_hint(&user, &scope_key, &state.project) {
                    hints.push(hint);
                }
                let scope_input = scope::SessionScope {
                    kind: scope_kind,
                    scope_key: scope_key.clone(),
                };
                if let Some(out) = scope::CurrentScopeHook.compute(&scope_input) {
                    hints.push(out.text);
                }
                if let Some(idea_hint) = title_hints::build_idea_hint(card_description.as_deref()) {
                    hints.push(idea_hint);
                }
                let mut req = duckchat::TitleRequest::new(user);
                req.context_hints = hints;
                let route_key = key.clone();
                let work = async move {
                    use duckchat::Provider;
                    let provider = duckchat::claude_code::ClaudeCodeProvider::new();
                    provider
                        .title_summary(req, &working_dir)
                        .await
                        .map_err(|e| e.to_string())
                };
                let title_task = Task::perform(work, move |result| Message::SessionTitleReady {
                    key: route_key.clone(),
                    result,
                });
                return Task::batch([snap_task, title_task]);
            }
            return snap_task;
        }
        Message::SessionTitleReady { key, result } => {
            let title = match result {
                Ok(t) if !t.trim().is_empty() => t.trim().to_string(),
                Ok(_) => {
                    tracing::warn!(key, "title summariser returned empty string");
                    return Task::none();
                }
                Err(e) => {
                    tracing::warn!(key, "title summary failed: {e}");
                    return Task::none();
                }
            };
            apply_session_title(state, &key, &title);
        }
        Message::ThemeChanged(mode) => {
            theme::set_mode(mode);
            return rehighlight_all(state);
        }
        Message::StreamTick => {
            widget::streaming_indicator::bump_tick();
        }
        Message::KeyPress(key, mods, text) => {
            // Cmd+P: open file finder.
            if mods.command() && key == keyboard::Key::Character("p".into()) {
                // Skip when no project is loaded — file finder needs a project
                // root to walk. Cmd+O is the open-project key in that case.
                if state.project.project_root.is_some() {
                    return update(state, Message::FileFinder(widget::file_finder::Msg::Open));
                }
            }

            // Cmd+O: open the project picker.
            if mods.command() && key == keyboard::Key::Character("o".into()) {
                return update(
                    state,
                    Message::ProjectPicker(widget::project_picker::Msg::Open),
                );
            }

            // Cmd+I: open the Quick Idea modal. Capture or jump to an idea
            // without leaving the current area; needs a project root because
            // the modal reads/writes ideas under `<data>/ideas/`.
            if mods.command()
                && matches!(&key, keyboard::Key::Character(c) if c.eq_ignore_ascii_case("i"))
                && state.project.project_root.is_some()
            {
                return update(state, Message::QuickIdea(widget::quick_idea::Msg::Open));
            }

            // Cmd+Shift+N: spawn another duckboard process. Iced is single-window
            // per-process, so a new "window" is a new instance — independent state,
            // file watcher, PTYs. Config writes race last-write-wins on quit.
            if mods.command()
                && mods.shift()
                && matches!(&key, keyboard::Key::Character(c) if c.eq_ignore_ascii_case("n"))
            {
                spawn_new_instance();
                return Task::none();
            }

            // Cmd+N in the Ideas area: add a new idea and open it. Area-scoped
            // so it doesn't fight the Change area's Cmd+N (which spawns a
            // chat session or exploration and is handled inside the
            // chat-focus block further down).
            if mods.command()
                && key == keyboard::Key::Character("n".into())
                && state.active_area == Area::Ideas
                && state.project.project_root.is_some()
            {
                return update(state, Message::Ideas(area::ideas::Message::AddIdea));
            }

            // Cmd+S in the Ideas area with the pinned tab active → save the
            // idea body (frontmatter is rederived from H1). Routed through
            // the ideas update so it picks up filename moves on title/tag
            // changes; the generic file-save path doesn't know about
            // frontmatter.
            if mods.command()
                && matches!(&key, keyboard::Key::Character(c) if c.eq_ignore_ascii_case("s"))
                && state.active_area == Area::Ideas
                && matches!(state.tabs.active, tab_bar::ActiveTab::Preview)
                && state
                    .tabs
                    .active_tab()
                    .is_some_and(|t| t.id.starts_with(area::ideas::PINNED_TAB_PREFIX))
            {
                return update(state, Message::Ideas(area::ideas::Message::SaveBody));
            }

            // Cmd+Shift+F: open project-wide text search.
            if mods.command()
                && mods.shift()
                && matches!(&key, keyboard::Key::Character(c) if c.eq_ignore_ascii_case("f"))
            {
                return update(state, Message::TextSearch(widget::text_search::Msg::Open));
            }

            // When text search is visible, route navigation keys.
            if state.text_search.visible {
                use keyboard::key::Named;
                match &key {
                    keyboard::Key::Named(Named::Escape) => {
                        let _ = update(
                            state,
                            Message::TextSearch(widget::text_search::Msg::Close),
                        );
                    }
                    keyboard::Key::Named(Named::Enter) => {
                        let msg = if mods.shift() {
                            widget::text_search::Msg::ConfirmStack
                        } else {
                            widget::text_search::Msg::ConfirmTop
                        };
                        // Must propagate the returned Task: Shift+Enter's
                        // `ConfirmStack` kicks off async highlight jobs that
                        // would be dropped if we discarded this.
                        return update(state, Message::TextSearch(msg));
                    }
                    keyboard::Key::Named(Named::ArrowDown) => {
                        let _ = update(
                            state,
                            Message::TextSearch(widget::text_search::Msg::SelectNext),
                        );
                    }
                    keyboard::Key::Named(Named::ArrowUp) => {
                        let _ = update(
                            state,
                            Message::TextSearch(widget::text_search::Msg::SelectPrev),
                        );
                    }
                    _ if mods.control() && key == keyboard::Key::Character("n".into()) => {
                        let _ = update(
                            state,
                            Message::TextSearch(widget::text_search::Msg::SelectNext),
                        );
                    }
                    _ if mods.control() && key == keyboard::Key::Character("p".into()) => {
                        let _ = update(
                            state,
                            Message::TextSearch(widget::text_search::Msg::SelectPrev),
                        );
                    }
                    _ => {}
                }
                return Task::none();
            }

            // When project picker is visible, route navigation keys.
            if state.project_picker.visible {
                use keyboard::key::Named;
                match &key {
                    keyboard::Key::Named(Named::Escape) => {
                        let _ = update(
                            state,
                            Message::ProjectPicker(widget::project_picker::Msg::Close),
                        );
                    }
                    keyboard::Key::Named(Named::Tab) => {
                        // Must propagate the Task — TabComplete returns a
                        // `move_cursor_to_end` operation that would be
                        // dropped by `let _ = ...`, leaving the caret in
                        // the middle of the freshly-completed path.
                        return update(
                            state,
                            Message::ProjectPicker(widget::project_picker::Msg::TabComplete),
                        );
                    }
                    keyboard::Key::Named(Named::Enter) => {
                        return update(
                            state,
                            Message::ProjectPicker(widget::project_picker::Msg::Confirm),
                        );
                    }
                    keyboard::Key::Named(Named::ArrowDown) => {
                        let _ = update(
                            state,
                            Message::ProjectPicker(widget::project_picker::Msg::SelectNext),
                        );
                    }
                    keyboard::Key::Named(Named::ArrowUp) => {
                        let _ = update(
                            state,
                            Message::ProjectPicker(widget::project_picker::Msg::SelectPrev),
                        );
                    }
                    _ if mods.control() && key == keyboard::Key::Character("n".into()) => {
                        let _ = update(
                            state,
                            Message::ProjectPicker(widget::project_picker::Msg::SelectNext),
                        );
                    }
                    _ if mods.control() && key == keyboard::Key::Character("p".into()) => {
                        let _ = update(
                            state,
                            Message::ProjectPicker(widget::project_picker::Msg::SelectPrev),
                        );
                    }
                    _ => {}
                }
                return Task::none();
            }

            // Esc closes the inline tag-add/edit input when it's open. Handled
            // ahead of the file finder block because the input is a
            // text_input — iced captures Escape to clear focus, but we want
            // it to also dismiss the input.
            if state.active_area == Area::Ideas
                && state.ideas.tag_input.is_some()
                && matches!(&key, keyboard::Key::Named(keyboard::key::Named::Escape))
            {
                return update(
                    state,
                    Message::Ideas(area::ideas::Message::CancelTagInput),
                );
            }

            // When the Quick Idea modal is visible, route Esc + ctrl-n/p.
            // Enter is consumed by the embedded TextEdit's `on_submit` and
            // never reaches this handler; cmd shortcuts and printable
            // characters likewise fall to the editor while it is focused.
            if state.quick_idea.visible {
                use keyboard::key::Named;
                match &key {
                    keyboard::Key::Named(Named::Escape) => {
                        // The inline tag-add input should swallow Esc itself
                        // before the modal closes.
                        let inner_msg = if state.quick_idea.tag_input.is_some() {
                            widget::quick_idea::Msg::CancelTagInput
                        } else {
                            widget::quick_idea::Msg::Close
                        };
                        return update(state, Message::QuickIdea(inner_msg));
                    }
                    _ if mods.control() && key == keyboard::Key::Character("n".into()) => {
                        let _ = update(
                            state,
                            Message::QuickIdea(widget::quick_idea::Msg::SelectNext),
                        );
                    }
                    _ if mods.control() && key == keyboard::Key::Character("p".into()) => {
                        let _ = update(
                            state,
                            Message::QuickIdea(widget::quick_idea::Msg::SelectPrev),
                        );
                    }
                    _ => {}
                }
                return Task::none();
            }

            // When file finder is visible, route navigation keys.
            if state.file_finder.visible {
                use keyboard::key::Named;
                match &key {
                    keyboard::Key::Named(Named::Escape) => {
                        let _ = update(state, Message::FileFinder(widget::file_finder::Msg::Close));
                    }
                    keyboard::Key::Named(Named::Enter) => {
                        // Must propagate the returned Task: Confirm opens a
                        // file tab and spawns its async highlight, which
                        // would be dropped by `let _ = ...`.
                        return update(
                            state,
                            Message::FileFinder(widget::file_finder::Msg::Confirm),
                        );
                    }
                    keyboard::Key::Named(Named::ArrowDown) => {
                        let _ = update(
                            state,
                            Message::FileFinder(widget::file_finder::Msg::SelectNext),
                        );
                    }
                    keyboard::Key::Named(Named::ArrowUp) => {
                        let _ = update(
                            state,
                            Message::FileFinder(widget::file_finder::Msg::SelectPrev),
                        );
                    }
                    _ if mods.control() && key == keyboard::Key::Character("n".into()) => {
                        let _ = update(
                            state,
                            Message::FileFinder(widget::file_finder::Msg::SelectNext),
                        );
                    }
                    _ if mods.control() && key == keyboard::Key::Character("p".into()) => {
                        let _ = update(
                            state,
                            Message::FileFinder(widget::file_finder::Msg::SelectPrev),
                        );
                    }
                    _ => {}
                }
                return Task::none();
            }

            // Get the active area's interaction state for keyboard routing.
            let active_info = state.active_interaction().map(|(i, _key)| {
                let agent_chat_active =
                    i.visible && i.active_tab == ActiveTab::Chat && i.active().is_some();
                let terminal_focused = i.terminal_focused;
                (agent_chat_active, terminal_focused)
            });
            // We need the key separately (can't hold borrow across mutable calls).
            let active_key = state.active_interaction_key();

            if let (Some((agent_chat_active, terminal_focused, ..)), Some(routing_key)) =
                (active_info, &active_key)
            {
                // Agent chat keyboard shortcuts (completion, esc-cancel, enter-send).
                if agent_chat_active {
                    if let Some(ix) = state.interaction_mut(routing_key) {
                        match interaction::handle_agent_chat_key(ix, &key, mods) {
                            interaction::AgentChatKeyResult::Handled => return Task::none(),
                            interaction::AgentChatKeyResult::Dispatch(msg) => {
                                return dispatch_interaction_msg(
                                    state,
                                    routing_key,
                                    interaction::Msg::AgentChat(msg),
                                );
                            }
                            interaction::AgentChatKeyResult::NotHandled => {}
                        }
                    }

                    // Cmd-N (Ctrl-N off-mac): spawn a fresh exploration when the
                    // current scope is an exploration, otherwise start a new
                    // chat session inside the current change. Explorations use
                    // the single-session UI, so reusing NewSession would just
                    // visually clear the current chat — and Clear is button-only
                    // by design.
                    if state.active_area == Area::Change
                        && mods.command()
                        && key == keyboard::Key::Character("n".into())
                    {
                        if state.change.is_exploration_selected() {
                            return Task::batch([
                                update(
                                    state,
                                    Message::Change(area::change::Message::AddExploration),
                                ),
                                focus_chat_input(),
                            ]);
                        }
                        return dispatch_interaction_msg(
                            state,
                            routing_key,
                            interaction::Msg::NewSession,
                        );
                    }
                }

                // Terminal keyboard capture.
                if terminal_focused {
                    // Clipboard shortcuts: Cmd+C/V on macOS, Ctrl+Shift+C/V elsewhere.
                    let clipboard_combo = if cfg!(target_os = "macos") {
                        mods.logo() && !mods.control() && !mods.alt() && !mods.shift()
                    } else {
                        mods.control() && mods.shift() && !mods.alt() && !mods.logo()
                    };
                    if clipboard_combo && let keyboard::Key::Character(c) = &key {
                        match c.as_str().to_ascii_lowercase().as_str() {
                            "c" => {
                                let selection = state
                                    .interaction_mut(routing_key)
                                    .and_then(|ix| ix.active_terminal())
                                    .and_then(|tt| tt.state.selection_text());
                                if let Some(text) = selection {
                                    return iced::clipboard::write(text);
                                }
                                return Task::none();
                            }
                            "v" => {
                                let route = routing_key.clone();
                                return iced::clipboard::read()
                                    .map(move |opt| Message::TerminalPaste(route.clone(), opt));
                            }
                            _ => {}
                        }
                    }

                    if let Some(ix) = state.interaction_mut(routing_key)
                        && let Some(tt) = ix.active_terminal_mut()
                    {
                        tt.state.write_key(key, mods, text.as_deref());
                    }
                }
            }
        }
    }
    take_pending_chat_snap(state)
}

/// Drain the `pending_snap_to_bottom` flag from any agent session and emit a
/// one-shot `snap_to_end` task. The flag is set in `send_prompt_text` when
/// the user submits while sticking to the bottom — without this, the user's
/// own message lands in the transcript before the first `AgentEvent` and
/// they don't see it auto-scroll.
fn take_pending_chat_snap(state: &mut State) -> Task<Message> {
    let mut should_snap = false;
    let mut clear = |ax: &mut interaction::AgentSession| {
        if ax.pending_snap_to_bottom {
            ax.pending_snap_to_bottom = false;
            should_snap = true;
        }
    };
    for ix in state.interactions.values_mut() {
        for ax in &mut ix.sessions {
            clear(ax);
        }
    }
    if should_snap {
        iced::widget::operation::snap_to_end(widget::agent_chat::CHAT_SCROLLABLE_ID)
    } else {
        Task::none()
    }
}

/// Resolve a composite routing key `<instance_id>/<session_id>` to its
/// AgentSession from the shared `interactions` map. Borrows only the map
/// so callers can keep parallel borrows on other `State` fields.
fn resolve_session_mut<'a>(
    interactions: &'a mut HashMap<scope::Scope, interaction::InteractionState>,
    key: &str,
) -> Option<&'a mut interaction::AgentSession> {
    let (ix_id_str, session_id) = key.split_once('/')?;
    let ix_id: u64 = ix_id_str.parse().ok()?;
    let ix = interactions
        .values_mut()
        .find(|ix| ix.instance_id == ix_id)?;
    ix.find_session_mut(session_id)
}

/// Dispatch an interaction message to the appropriate area by routing key.
fn dispatch_interaction_msg(state: &mut State, key: &str, msg: interaction::Msg) -> Task<Message> {
    match key {
        KEY_CAPS => update(state, Message::Caps(area::caps::Message::Interaction(msg))),
        KEY_CODEX => update(
            state,
            Message::Codex(area::codex::Message::Interaction(msg)),
        ),
        _ => {
            // Ideas-owned scope (exploration id; post-promotion the change
            // area takes over). For non-ideas scopes fall through to change.
            if state.ideas.idea_for_scope(key).is_some() {
                update(state, Message::Ideas(area::ideas::Message::Interaction(msg)))
            } else {
                update(
                    state,
                    Message::Change(area::change::Message::Interaction(msg)),
                )
            }
        }
    }
}

/// Focus the chat input. Used after creating, switching, or clearing a
/// session so the user can immediately type — no extra click required.
fn focus_chat_input() -> Task<Message> {
    iced::widget::operation::focus(widget::agent_chat::CHAT_INPUT_ID)
}

/// Persist the Quick Idea modal's buffer + tags through the existing
/// idea_store. Updates `state.ideas.ideas` and re-targets any open pinned
/// tab if the loaded idea's path moved on disk (title or primary-tag rename
/// triggers a `save_idea` rename).
fn save_quick_idea(
    state: &mut State,
    payload: widget::quick_idea::SavePayload,
    project_root: Option<&Path>,
) {
    if let Some(loaded_path) = payload.loaded_path {
        let Some(idx) = state
            .ideas
            .ideas
            .iter()
            .position(|i| i.abs_path == loaded_path)
        else {
            tracing::warn!("quick idea: loaded path no longer in ideas list");
            return;
        };
        let mut idea = state.ideas.ideas[idx].clone();
        idea.frontmatter.tags = payload.tags;
        if let Err(e) = idea_store::save_idea(&mut idea, &payload.body, project_root) {
            tracing::warn!("quick idea save failed: {e}");
            return;
        }
        let new_path = idea.abs_path.clone();
        let new_title = idea.display_title();
        state.ideas.ideas[idx] = idea;
        state
            .ideas
            .ideas
            .sort_by(|a, b| b.frontmatter.created.cmp(&a.frontmatter.created));
        if loaded_path != new_path {
            area::ideas::refresh_after_move(
                &mut state.ideas,
                &mut state.tabs,
                &loaded_path,
                &new_path,
                &new_title,
            );
        }
    } else {
        let mut idea = idea_store::new_idea();
        idea.frontmatter.tags = payload.tags;
        // Quick-capture flow: if the user typed a bare line (no H1), promote
        // it so `derive_title_from_body` lifts a real title and the file
        // doesn't land as `idea.md`.
        let body = ensure_h1_prefix(&payload.body);
        if let Err(e) = idea_store::save_idea(&mut idea, &body, project_root) {
            tracing::warn!("quick idea save failed: {e}");
            return;
        }
        state.ideas.ideas.push(idea);
        state
            .ideas
            .ideas
            .sort_by(|a, b| b.frontmatter.created.cmp(&a.frontmatter.created));
    }
}

/// Return `body` unchanged when it already opens with an H1. Otherwise:
/// short first lines (<= TITLE_PROMOTE_MAX_WORDS) get promoted in place by
/// prepending `# `; longer ones get a synthesized H1 from their first
/// SYNTHETIC_TITLE_WORDS words tacked on above the original body, so the
/// user's freeform paragraph still reads naturally below a usable title.
/// All-blank input is left as-is — `save_idea` falls back to `fallback_title`.
fn ensure_h1_prefix(body: &str) -> String {
    const TITLE_PROMOTE_MAX_WORDS: usize = 8;
    const SYNTHETIC_TITLE_WORDS: usize = 5;

    if idea_store::derive_title_from_body(body).is_some() {
        return body.to_string();
    }
    let first = body
        .lines()
        .find(|l| !l.trim().is_empty())
        .map(str::trim)
        .unwrap_or("");
    if first.is_empty() {
        return body.to_string();
    }
    let word_count = first.split_whitespace().count();
    if word_count <= TITLE_PROMOTE_MAX_WORDS {
        let mut promoted = false;
        let mut out = String::with_capacity(body.len() + 2);
        for line in body.split_inclusive('\n') {
            if !promoted && !line.trim().is_empty() {
                out.push_str("# ");
                out.push_str(line.trim_start());
                promoted = true;
            } else {
                out.push_str(line);
            }
        }
        return out;
    }
    let title: String = first
        .split_whitespace()
        .take(SYNTHETIC_TITLE_WORDS)
        .collect::<Vec<_>>()
        .join(" ");
    let mut out = String::with_capacity(body.len() + title.len() + 6);
    out.push_str("# ");
    out.push_str(&title);
    out.push_str("\n\n");
    out.push_str(body);
    out
}

/// True when `message` represents a user action that pulls focus away from
/// the inline tag-add/edit input — clicking the editor, switching areas,
/// triggering a lifecycle action on the idea, or selecting another idea.
/// Tag-related messages (the input's own keystrokes, chip clicks, + Tag,
/// etc.) explicitly keep the input alive.
fn tag_input_loses_focus_on(message: &Message) -> bool {
    use area::ideas::Message as IM;
    match message {
        Message::TabContent(_) => true,
        Message::AreaSelected(_) => true,
        Message::FileFinder(_) => true,
        Message::ProjectPicker(_) => true,
        Message::TextSearch(_) => true,
        Message::QuickIdea(_) => true,
        Message::Ideas(im) => matches!(
            im,
            IM::SelectIdea(_)
                | IM::AddIdea
                | IM::DeleteIdea(_)
                | IM::ArchiveIdea(_)
                | IM::UnarchiveIdea(_)
                | IM::StartExploration(_)
                | IM::OpenChange(_)
                | IM::ToggleSection(_)
                | IM::ToggleTagNode(_)
                | IM::SaveBody
                | IM::Interaction(_)
        ),
        _ => false,
    }
}

/// True when an interaction message changes the active session in a way that
/// should re-focus the chat input (new session created, current cleared).
fn is_chat_focus_msg(msg: Option<&interaction::Msg>) -> bool {
    matches!(
        msg,
        Some(interaction::Msg::NewSession | interaction::Msg::ClearSession)
    )
}

fn extract_change_interaction_msg(msg: &area::change::Message) -> Option<&interaction::Msg> {
    if let area::change::Message::Interaction(m) = msg {
        Some(m)
    } else {
        None
    }
}

fn extract_caps_interaction_msg(msg: &area::caps::Message) -> Option<&interaction::Msg> {
    if let area::caps::Message::Interaction(m) = msg {
        Some(m)
    } else {
        None
    }
}

fn extract_codex_interaction_msg(msg: &area::codex::Message) -> Option<&interaction::Msg> {
    if let area::codex::Message::Interaction(m) = msg {
        Some(m)
    } else {
        None
    }
}

fn extract_ideas_interaction_msg(msg: &area::ideas::Message) -> Option<&interaction::Msg> {
    if let area::ideas::Message::Interaction(m) = msg {
        Some(m)
    } else {
        None
    }
}

/// Re-highlight all open tabs and chat editors (e.g. after a theme switch).
///
/// `EditorState::highlight_spans` bake in concrete RGB colors at highlight
/// time, so a theme switch is invisible until every editor is re-highlighted.
///
/// File and diff tabs spawn async jobs (returned as a batched `Task`) so a
/// theme toggle doesn't block the UI while syntect reparses every open
/// file. Chat/queue buffers are small and stay sync — their highlight
/// cost is negligible.
fn rehighlight_all(state: &mut State) -> Task<Message> {
    let mut tasks: Vec<Task<Message>> = Vec::new();

    let area = state.active_area;
    let tabs = &mut state.tabs;
    let all_tabs = tabs.preview.iter_mut().chain(tabs.file_tabs.iter_mut());
    for tab in all_tabs {
        let tab_id = tab.id.clone();
        match &mut tab.view {
            tab_bar::TabView::Editor { editor, .. } => {
                tasks.push(spawn_file_tab_highlight(
                    area,
                    tab_id,
                    editor,
                    state.highlighter.clone(),
                    false,
                ));
            }
            tab_bar::TabView::Diff {
                editor,
                path,
                diff_data,
                ..
            } => {
                editor.highlight_version = editor.highlight_version.wrapping_add(1);
                editor.highlight_spans =
                    Some(widget::diff_view::build_diff_spans(diff_data, None));
                tasks.push(spawn_diff_highlight(
                    area,
                    tab_id,
                    editor.highlight_version,
                    path,
                    diff_data.clone(),
                    state.highlighter.clone(),
                ));
            }
            tab_bar::TabView::SearchStack { slices, .. } => {
                for slice in slices.iter_mut() {
                    let id = format!("file:{}", slice.rel_path);
                    rehighlight(&mut slice.editor, &id, &state.highlighter);
                }
            }
        }
    }

    // Cached previews from other areas don't render right now, but their
    // editor state survives the area switch and should also be refreshed.
    for slot in state.cached_previews.values_mut() {
        if let Some(tab) = slot.as_mut()
            && let tab_bar::TabView::Editor { editor, .. } = &mut tab.view
        {
            editor.highlight_version = editor.highlight_version.wrapping_add(1);
            rehighlight(editor, &tab.id, &state.highlighter);
        }
    }

    let md_syntax = state.highlighter.find_syntax("md");
    for ix in state.interactions.values_mut() {
        for ax in ix.sessions.iter_mut() {
            ax.chat_input.highlight_spans = Some(
                state
                    .highlighter
                    .highlight_lines(&ax.chat_input.lines, md_syntax),
            );
            for editor in ax.chat_editors.iter_mut() {
                editor.highlight_spans = Some(
                    state
                        .highlighter
                        .highlight_lines(&editor.lines, md_syntax),
                );
            }
        }
    }

    Task::batch(tasks)
}

/// Reload `ProjectData` and reconcile duckboard-local state: promote a selected
/// exploration if a new change appeared, migrate subscriptions when a change
/// was archived externally, and refresh the obvious-command hint. Returns
/// `true` when tab IDs were rewritten for an external archival, so the caller
/// can refresh open-tab contents from disk.
fn reload_and_reconcile(state: &mut State) -> bool {
    use std::collections::HashSet;

    let old_change_names: HashSet<String> = state
        .project
        .active_changes
        .iter()
        .map(|c| c.name.clone())
        .collect();
    let old_archived_names: HashSet<String> = state
        .project
        .archived_changes
        .iter()
        .map(|c| c.name.clone())
        .collect();

    state.project.reload();

    // Detect new change directories and promote exploration if active.
    if state.change.is_exploration_selected() {
        let new_change = state
            .project
            .active_changes
            .iter()
            .find(|c| !old_change_names.contains(&c.name))
            .map(|c| c.name.clone());

        if let Some(new_name) = new_change
            && let Some(exploration_id) = state.change.selected_change.clone()
        {
            tracing::info!(
                from = exploration_id,
                to = new_name.as_str(),
                "promoting exploration to real change"
            );
            area::change::promote_exploration(
                &mut state.change,
                &mut state.interactions,
                &exploration_id,
                &new_name,
                state.project.project_root.as_deref(),
            );
        }
    } else if let Some(idea_path) = state.ideas.selected.clone() {
        // Idea-owned exploration promotion. Same heuristic as the Changes
        // area: if the selected idea has an unpromoted exploration and a
        // new change directory appeared, bind it.
        let idea_needs_change = state
            .ideas
            .ideas
            .iter()
            .find(|i| i.abs_path == idea_path)
            .map(|i| i.frontmatter.exploration.is_some() && i.frontmatter.change.is_none())
            .unwrap_or(false);
        if idea_needs_change
            && let Some(new_name) = state
                .project
                .active_changes
                .iter()
                .find(|c| !old_change_names.contains(&c.name))
                .map(|c| c.name.clone())
        {
            let exploration_id = state
                .ideas
                .ideas
                .iter()
                .find(|i| i.abs_path == idea_path)
                .and_then(|i| i.frontmatter.exploration.clone());
            tracing::info!(
                idea = %idea_path.display(),
                to = new_name.as_str(),
                "promoting idea-owned exploration to real change"
            );
            promote_idea_exploration(state, &idea_path, &new_name);
            if let Some(exp_id) = exploration_id {
                state.change.explorations.retain(|e| e.id != exp_id);
                chat_store::save_explorations(
                    &state.change.explorations,
                    state.change.exploration_counter,
                    state.project.project_root.as_deref(),
                );
            }
        }
    }

    // Detect new archived change directories and migrate subscriptions from
    // the matching active-change name (archival happened externally).
    let new_archived: Vec<String> = state
        .project
        .archived_changes
        .iter()
        .filter(|c| !old_archived_names.contains(&c.name))
        .map(|c| c.name.clone())
        .collect();

    let mut archived_any = false;
    for archived_name in new_archived {
        let Some(base_name) = data::strip_archive_prefix(&archived_name) else {
            continue;
        };
        if state
            .interactions
            .contains_key(&scope::Scope::Change(base_name.to_string()))
        {
            tracing::info!(
                from = base_name,
                to = archived_name.as_str(),
                "migrating subscriptions to archived change"
            );
            area::change::archive_change(
                &mut state.change,
                &mut state.interactions,
                &mut state.tabs,
                base_name,
                &archived_name,
                state.project.project_root.as_deref(),
            );
            archived_any = true;
        }
    }

    area::change::refresh_obvious_command(
        &state.change,
        &mut state.interactions,
        &state.project,
    );
    archived_any
}

/// Migrate an idea-owned exploration's interaction state from
/// `Scope::Exploration(exp_id)` to `Scope::Change(new_name)`. Stamps the
/// idea's frontmatter, transitions its file into the Change subtree, and
/// renames the on-disk chats directory.
fn promote_idea_exploration(state: &mut State, idea_path: &Path, change_name: &str) {
    let project_root = state.project.project_root.clone();
    let (exploration_id, moved) = {
        let Some(idea) = state
            .ideas
            .ideas
            .iter_mut()
            .find(|i| i.abs_path == idea_path)
        else {
            return;
        };
        let Some(exp_id) = idea.frontmatter.exploration.clone() else {
            return;
        };
        idea.frontmatter.change = Some(change_name.to_string());
        idea.state = idea_store::IdeaState::Change;
        let body = idea_store::read_body(&idea.abs_path).unwrap_or_default();
        if let Err(e) = idea_store::save_idea(idea, &body, project_root.as_deref()) {
            tracing::warn!("failed to persist idea on promotion: {e}");
        }
        (exp_id, (idea.abs_path.clone(), idea.display_title()))
    };

    let (new_path, new_title) = moved;
    area::ideas::refresh_after_move(
        &mut state.ideas,
        &mut state.tabs,
        idea_path,
        &new_path,
        &new_title,
    );

    if let Some(mut ix) = state
        .interactions
        .remove(&scope::Scope::Exploration(exploration_id.clone()))
    {
        for ax in ix.sessions.iter_mut() {
            ax.session.scope = change_name.to_string();
            ax.scope_kind = scope::ScopeKind::Change;
        }
        interaction::reconcile_display_names(&mut ix.sessions, change_name);
        state
            .interactions
            .insert(scope::Scope::Change(change_name.to_string()), ix);
    }
    chat_store::rename_scope(&exploration_id, change_name, project_root.as_deref());
}

/// Re-read content for all open text tabs from disk and enqueue async
/// highlight jobs so the refresh doesn't block the UI.
fn refresh_open_tabs(state: &mut State, tasks: &mut Vec<Task<Message>>) {
    let area = state.active_area;
    let refresh_tab = |tab: &mut tab_bar::Tab,
                       project: &ProjectData,
                       highlighter: &Arc<highlight::SyntaxHighlighter>,
                       tasks: &mut Vec<Task<Message>>| {
        let tab_id = tab.id.clone();
        if let tab_bar::TabView::Editor { editor, .. } = &mut tab.view
            && let Some(content) = project.read_artifact(&tab_id)
        {
            let mut next = widget::text_edit::EditorState::new(&content);
            next.highlight_version = editor.highlight_version.wrapping_add(1);
            *editor = next;
            tasks.push(spawn_file_tab_highlight(
                area,
                tab_id,
                editor,
                highlighter.clone(),
                false,
            ));
        }
    };

    let project = &state.project;
    let highlighter = &state.highlighter;
    let all_tabs = state
        .tabs
        .preview
        .iter_mut()
        .chain(state.tabs.file_tabs.iter_mut());
    for tab in all_tabs {
        refresh_tab(tab, project, highlighter, tasks);
    }
    for slot in state.cached_previews.values_mut() {
        if let Some(tab) = slot.as_mut() {
            refresh_tab(tab, project, highlighter, tasks);
        }
    }
}

/// Apply an editor action to the active tab's editor state. Returns a
/// debounced async highlight task when the action mutates content; the
/// caller must propagate it up to the runtime or the spans will never
/// refresh. Non-mutating actions (cursor moves, scroll, save) return
/// `Task::none()`.
fn handle_editor_action(
    tabs: &mut tab_bar::TabState,
    area: Area,
    action: widget::text_edit::EditorAction,
    highlighter: Arc<highlight::SyntaxHighlighter>,
) -> Task<Message> {
    let tab = match tabs.active_tab_mut() {
        Some(t) => t,
        None => return Task::none(),
    };

    if matches!(action, widget::text_edit::EditorAction::SaveRequested) {
        if let tab_bar::TabView::Editor { editor, path } = &mut tab.view
            && let Some(path) = path.as_ref()
        {
            let text = editor.text();
            match std::fs::write(path, &text) {
                Ok(()) => {
                    editor.dirty = false;
                    tracing::info!(path = %path.display(), "saved file");
                }
                Err(err) => {
                    tracing::error!(path = %path.display(), %err, "failed to save file");
                }
            }
        }
        return Task::none();
    }

    if let widget::text_edit::EditorAction::OpenUrl(url) = &action {
        if let Err(err) = opener::open(url) {
            tracing::warn!(%url, %err, "failed to open editor URL");
        }
        return Task::none();
    }

    let tab_id = tab.id.clone();
    let (editor, is_diff) = match &mut tab.view {
        tab_bar::TabView::Editor { editor, .. } => (editor, false),
        tab_bar::TabView::Diff { editor, .. } => (editor, true),
        tab_bar::TabView::SearchStack { .. } => return Task::none(),
    };

    if editor.apply_action(action) {
        // Diff tabs are read-only, so `apply_action` shouldn't return true
        // for them. Guard anyway: a future editable-diff variant would break
        // silently otherwise.
        if is_diff {
            return Task::none();
        }
        spawn_file_tab_highlight(area, tab_id, editor, highlighter, true)
    } else {
        Task::none()
    }
}

/// (Re-)compute syntax highlighting for the given editor state.
pub fn rehighlight(
    editor: &mut widget::text_edit::EditorState,
    tab_id: &str,
    highlighter: &highlight::SyntaxHighlighter,
) {
    let path_str = tab_id.strip_prefix("file:").unwrap_or(tab_id);
    let ext = std::path::Path::new(path_str)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("txt");
    let syntax = highlighter.find_syntax(ext);
    editor.highlight_spans = Some(highlighter.highlight_lines(&editor.lines, syntax));
}

/// Pause before running the blocking highlight so that a burst of edits
/// doesn't spawn one 500ms syntect job per keystroke. Stale results are
/// also dropped by the version check, but `spawn_blocking` can't be
/// cancelled — so the sleep saves wasted CPU on throwaway work.
const HIGHLIGHT_DEBOUNCE_MS: u64 = 150;

/// Kick off an async syntax-highlight for an editable file tab. The
/// current `highlight_version` is snapshotted at spawn time and echoed back
/// in [`Message::FileTabHighlighted`]; the handler only applies the spans
/// if the editor's version still matches, so edits during the highlight
/// window simply drop the result.
fn spawn_file_tab_highlight(
    area: Area,
    tab_id: String,
    editor: &widget::text_edit::EditorState,
    highlighter: Arc<highlight::SyntaxHighlighter>,
    debounce: bool,
) -> Task<Message> {
    let version = editor.highlight_version;
    let lines = editor.lines.clone();
    let path_str = tab_id.strip_prefix("file:").unwrap_or(&tab_id).to_string();
    let ext = std::path::Path::new(&path_str)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("txt")
        .to_string();
    let delay = if debounce {
        std::time::Duration::from_millis(HIGHLIGHT_DEBOUNCE_MS)
    } else {
        std::time::Duration::ZERO
    };
    Task::perform(
        async move {
            if !delay.is_zero() {
                tokio::time::sleep(delay).await;
            }
            tokio::task::spawn_blocking(move || {
                let syntax = highlighter.find_syntax(&ext);
                highlighter.highlight_lines(&lines, syntax)
            })
            .await
            .unwrap_or_default()
        },
        move |spans| Message::FileTabHighlighted {
            area,
            tab_id,
            version,
            spans: Arc::new(spans),
        },
    )
}

/// Kick off an async syntect highlight for both sides of a diff. The
/// handler rebuilds `editor.highlight_spans` via
/// [`widget::diff_view::build_diff_spans`] when the version still matches.
fn spawn_diff_highlight(
    area: Area,
    tab_id: String,
    version: u64,
    rel_path: &std::path::Path,
    diff_data: Arc<vcs::DiffData>,
    highlighter: Arc<highlight::SyntaxHighlighter>,
) -> Task<Message> {
    let ext = rel_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("txt")
        .to_string();
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || {
                widget::diff_view::compute_diff_highlight(&diff_data, &ext, &highlighter)
            })
            .await
            .unwrap_or_else(|_| widget::diff_view::DiffHighlight {
                old_spans: Vec::new(),
                new_spans: Vec::new(),
            })
        },
        move |highlight| Message::DiffTabHighlighted {
            area,
            tab_id,
            version,
            highlight: Arc::new(highlight),
        },
    )
}

/// Open a diff tab for `rel_path` in the given area's preview slot, then
/// return the async task that computes its syntect highlight. The tab
/// renders with fallback muted colors until the task completes.
fn open_diff_preview(state: &mut State, area: Area, rel_path: &std::path::Path) -> Task<Message> {
    let Some(root) = state.project.project_root.as_deref() else {
        return Task::none();
    };
    let Some(content) = widget::diff_view::build_diff_tab(root, rel_path) else {
        return Task::none();
    };
    let id = format!("vcs:{}", rel_path.display());
    let title = rel_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| rel_path.display().to_string());
    let diff_data = content.diff_data.clone();
    let path_for_task = rel_path.to_path_buf();

    state.tabs.open_diff(
        id.clone(),
        title,
        content.editor,
        content.path,
        content.status,
        content.diff_data,
    );

    let version = state
        .tabs
        .preview
        .as_ref()
        .and_then(|t| {
            if let tab_bar::TabView::Diff { editor, .. } = &t.view {
                Some(editor.highlight_version)
            } else {
                None
            }
        })
        .unwrap_or(0);

    spawn_diff_highlight(
        area,
        id,
        version,
        &path_for_task,
        diff_data,
        state.highlighter.clone(),
    )
}

/// Walk the active area's `TabState` to find a file tab (preview or
/// `file_tabs`) by id. Returns a mutable reference to the editor if the
/// tab exists and is an `Editor` view.
fn find_editor_mut<'a>(
    tabs: &'a mut tab_bar::TabState,
    tab_id: &str,
) -> Option<&'a mut widget::text_edit::EditorState> {
    let tab = tabs
        .preview
        .as_mut()
        .filter(|t| t.id == tab_id)
        .or_else(|| tabs.file_tabs.iter_mut().find(|t| t.id == tab_id))?;
    match &mut tab.view {
        tab_bar::TabView::Editor { editor, .. } => Some(editor),
        _ => None,
    }
}

/// Like `find_editor_mut` but for `Diff` tabs. Returns the editor plus the
/// `DiffData` needed to rebuild composite per-line spans.
fn find_diff_tab_mut<'a>(
    tabs: &'a mut tab_bar::TabState,
    tab_id: &str,
) -> Option<(
    &'a mut widget::text_edit::EditorState,
    Arc<vcs::DiffData>,
)> {
    let tab = tabs
        .preview
        .as_mut()
        .filter(|t| t.id == tab_id)
        .or_else(|| tabs.file_tabs.iter_mut().find(|t| t.id == tab_id))?;
    match &mut tab.view {
        tab_bar::TabView::Diff {
            editor, diff_data, ..
        } => Some((editor, diff_data.clone())),
        _ => None,
    }
}

/// Refresh the VCS changed files list.
fn refresh_changed_files(state: &mut State) {
    if let Some(root) = &state.project.project_root {
        state.change.set_changed_files(vcs::changed_files(root));
    }
}

/// Re-read any open `file:`-prefixed tabs whose underlying path matches
/// `changed_path`. Used when the watcher reports a file modification.
/// Re-read an artifact (duckspec-tracked file) into any open preview /
/// file tabs across all three areas, then enqueue async highlight jobs.
fn refresh_artifact_tabs(
    state: &mut State,
    id: &str,
    content: String,
    tasks: &mut Vec<Task<Message>>,
) {
    let area = state.active_area;
    if let Some(editor) = state.tabs.refresh_content(id, content.clone()) {
        tasks.push(spawn_file_tab_highlight(
            area,
            id.to_string(),
            editor,
            state.highlighter.clone(),
            false,
        ));
    }
    refresh_cached_artifact_tabs(state, id, &content);
}

/// Replace cached preview content for tabs that match `id` in any non-active
/// area's stashed slot. Reuses `EditorState::new` so cursor/dirty don't
/// outlive the file rewrite.
fn refresh_cached_artifact_tabs(state: &mut State, id: &str, content: &str) {
    for slot in state.cached_previews.values_mut() {
        if let Some(tab) = slot.as_mut()
            && tab.id == id
            && let tab_bar::TabView::Editor { editor, .. } = &mut tab.view
        {
            *editor = widget::text_edit::EditorState::new(content);
        }
    }
}

fn refresh_file_tabs_for_path(
    state: &mut State,
    project_root: &std::path::Path,
    changed_path: &std::path::Path,
    tasks: &mut Vec<Task<Message>>,
) {
    let Ok(rel) = changed_path.strip_prefix(project_root) else {
        return;
    };
    let id = format!("file:{}", rel.display());
    let Ok(content) = std::fs::read_to_string(changed_path) else {
        return;
    };
    let area = state.active_area;
    if let Some(editor) = state.tabs.refresh_content(&id, content.clone()) {
        tasks.push(spawn_file_tab_highlight(
            area,
            id.clone(),
            editor,
            state.highlighter.clone(),
            false,
        ));
    }
    refresh_cached_artifact_tabs(state, &id, &content);
}

/// Rebuild any open `vcs:`-prefixed tabs whose underlying path matches
/// `changed_path`. If the file no longer differs from HEAD, close the tab.
fn refresh_diff_tabs_for_path(
    state: &mut State,
    project_root: &std::path::Path,
    changed_path: &std::path::Path,
    tasks: &mut Vec<Task<Message>>,
) {
    let Ok(rel) = changed_path.strip_prefix(project_root) else {
        return;
    };
    let id = format!("vcs:{}", rel.display());
    rebuild_diff_tab(state, project_root, &id, rel, tasks);
}

/// Rebuild every open diff tab — used on VCS state changes.
fn refresh_all_diff_tabs(
    state: &mut State,
    project_root: &std::path::Path,
    tasks: &mut Vec<Task<Message>>,
) {
    let ids: Vec<String> = state
        .tabs
        .preview
        .iter()
        .chain(state.tabs.file_tabs.iter())
        .filter(|t| matches!(t.view, tab_bar::TabView::Diff { .. }))
        .map(|t| t.id.clone())
        .collect();
    for id in ids {
        let Some(rel_str) = id.strip_prefix("vcs:") else {
            continue;
        };
        let rel = std::path::PathBuf::from(rel_str);
        rebuild_diff_tab(state, project_root, &id, &rel, tasks);
    }
}

fn rebuild_diff_tab(
    state: &mut State,
    project_root: &std::path::Path,
    id: &str,
    rel: &std::path::Path,
    tasks: &mut Vec<Task<Message>>,
) {
    match widget::diff_view::build_diff_tab(project_root, rel) {
        Some(content) => {
            let area = state.active_area;
            state.tabs.refresh_diff(
                id,
                content.editor.clone(),
                content.path.clone(),
                content.status,
                content.diff_data.clone(),
            );
            if let Some((editor, _)) = find_diff_tab_mut(&mut state.tabs, id) {
                editor.highlight_version = editor.highlight_version.wrapping_add(1);
                let version = editor.highlight_version;
                tasks.push(spawn_diff_highlight(
                    area,
                    id.to_string(),
                    version,
                    rel,
                    content.diff_data.clone(),
                    state.highlighter.clone(),
                ));
            }
        }
        None => {
            state.tabs.close_by_id(id);
            close_cached_tabs(state, id);
        }
    }
}

// ── Text search helpers ─────────────────────────────────────────────────────

/// Bump the query id and spawn a background search, returning a Task whose
/// completion dispatches `ResultsReady` with that id. Stale results are
/// discarded by the handler based on the id.
fn spawn_text_search(state: &mut State, query: String) -> Task<Message> {
    let Some(root) = state.project.project_root.clone() else {
        return Task::none();
    };
    state.text_search.latest_query_id += 1;
    let id = state.text_search.latest_query_id;
    state.text_search.searching = true;
    let scope = state.text_search.scope;
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || {
                widget::text_search::search_blocking(root, query, scope)
            })
            .await
            .unwrap_or_default()
        },
        move |results| Message::TextSearch(widget::text_search::Msg::ResultsReady(id, results)),
    )
}

/// Force the active area to Change when a file is opened from a non-area
/// context (Dashboard, Settings) — those areas don't render tabs themselves.
fn ensure_active_area(active_area: &mut Area) {
    if matches!(*active_area, Area::Dashboard | Area::Settings) {
        *active_area = Area::Change;
    }
}

/// Close a tab id from every cached per-area preview slot. Only the active
/// area's preview lives in `state.tabs.preview`; the others are stashed in
/// `state.cached_previews`. When a file disappears from disk we need to
/// evict matching previews from both.
fn close_cached_tabs(state: &mut State, id: &str) {
    for slot in state.cached_previews.values_mut() {
        if let Some(tab) = slot.as_ref()
            && tab.id == id
        {
            *slot = None;
        }
    }
}

/// Switch areas, snapshotting the current pinned tab + active-tab pointer
/// into the per-area cache and restoring the new area's cached values.
/// File tabs stay in `state.tabs` and persist across the swap; per-area
/// list selection is held by each area's own `selected` field, so list
/// highlighting survives switching to a file tab and back.
fn switch_area(state: &mut State, target: Area) {
    if state.active_area == target {
        return;
    }
    let prev = state.active_area;
    state
        .cached_previews
        .insert(prev, state.tabs.preview.take());
    state.cached_active.insert(prev, state.tabs.active);

    state.tabs.preview = state
        .cached_previews
        .remove(&target)
        .unwrap_or(None);
    state.tabs.active = state
        .cached_active
        .remove(&target)
        .unwrap_or(tab_bar::ActiveTab::Preview);

    // Clamp to a valid tab in case the cached pointer is stale (preview
    // gone, or file tab index out of range).
    match state.tabs.active {
        tab_bar::ActiveTab::Preview if state.tabs.preview.is_none() => {
            state.tabs.active = if state.tabs.file_tabs.is_empty() {
                tab_bar::ActiveTab::Preview
            } else {
                tab_bar::ActiveTab::File(0)
            };
        }
        tab_bar::ActiveTab::File(i) if i >= state.tabs.file_tabs.len() => {
            state.tabs.active = if state.tabs.preview.is_some() {
                tab_bar::ActiveTab::Preview
            } else if state.tabs.file_tabs.is_empty() {
                tab_bar::ActiveTab::Preview
            } else {
                tab_bar::ActiveTab::File(state.tabs.file_tabs.len() - 1)
            };
        }
        _ => {}
    }
    state.active_area = target;
}

/// Route a top-level `Message::Interaction` to the active area's update fn.
/// Single source of truth so chat/terminal events fall into the right
/// per-area session-management semantics (multi-session for Change, etc).
fn route_interaction(state: &mut State, msg: interaction::Msg) -> Task<Message> {
    let needs_focus = is_chat_focus_msg(Some(&msg));
    match state.active_area {
        Area::Change => {
            area::change::update(
                &mut state.change,
                &mut state.tabs,
                &mut state.interactions,
                area::change::Message::Interaction(msg),
                &state.project,
                &state.highlighter,
            );
        }
        Area::Caps => {
            let ix = state
                .interactions
                .entry(scope::Scope::Caps)
                .or_default();
            area::caps::update(
                &mut state.caps,
                &mut state.tabs,
                ix,
                area::caps::Message::Interaction(msg),
                &state.project,
                &state.highlighter,
            );
        }
        Area::Codex => {
            let ix = state
                .interactions
                .entry(scope::Scope::Codex)
                .or_default();
            area::codex::update(
                &mut state.codex,
                &mut state.tabs,
                ix,
                area::codex::Message::Interaction(msg),
                &state.project,
                &state.highlighter,
            );
        }
        Area::Ideas => {
            area::ideas::update(
                &mut state.ideas,
                &mut state.tabs,
                &mut state.interactions,
                area::ideas::Message::Interaction(msg),
                &state.project,
                &state.highlighter,
            );
        }
        Area::Dashboard | Area::Settings => {}
    }
    if needs_focus { focus_chat_input() } else { Task::none() }
}

/// Tag a set of line indices with `LineBgKind::Match` so they stand out
/// against the syntax-highlighted body. Used when opening a file from any
/// search flow (search overlay top-match, search-stack slice header) and,
/// later, by the planned per-file search feature — the mechanism is agnostic
/// to which search populated the list.
pub fn set_match_line_highlights(editor: &mut widget::text_edit::EditorState, lines: &[usize]) {
    if editor.line_backgrounds.len() != editor.lines.len() {
        editor.line_backgrounds = vec![None; editor.lines.len()];
    }
    for &line in lines {
        if let Some(slot) = editor.line_backgrounds.get_mut(line) {
            *slot = Some(widget::text_edit::LineBgKind::Match);
        }
    }
}

/// Open a single search hit as a regular file tab, scrolled so the match line
/// sits near the center of the editor viewport. Highlights every hit in
/// `all_hits` whose path matches this file so the user sees the full picture
/// rather than just the one they confirmed.
fn open_search_hit_as_file(
    state: &mut State,
    hit: &widget::text_search::SearchHit,
    all_hits: &[widget::text_search::SearchHit],
) -> Task<Message> {
    let Ok(content) = std::fs::read_to_string(&hit.abs_path) else {
        return Task::none();
    };
    let id = format!("file:{}", hit.rel_path);
    let title = std::path::Path::new(&hit.rel_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| hit.rel_path.clone());
    let line = hit.line;
    let match_lines: Vec<usize> = all_hits
        .iter()
        .filter(|h| h.rel_path == hit.rel_path)
        .map(|h| h.line)
        .collect();
    ensure_active_area(&mut state.active_area);
    let area = state.active_area;
    let highlighter = state.highlighter.clone();
    state
        .tabs
        .open_file(id.clone(), title, content, Some(hit.abs_path.clone()));
    if let Some(tab) = state.tabs.file_tabs.iter_mut().find(|t| t.id == id)
        && let tab_bar::TabView::Editor { editor, .. } = &mut tab.view
    {
        set_match_line_highlights(editor, &match_lines);
        let target_y = line as f32 * 20.0 - 300.0;
        editor.scroll_y = target_y.max(0.0);
        editor.cursor = widget::text_edit::Pos::new(line, 0);
        spawn_file_tab_highlight(area, id, editor, highlighter, false)
    } else {
        Task::none()
    }
}

/// Lines past the hit that a slice might reveal. Slices are fixed at 10
/// visible lines with the hit near the center, so highlighting the file up
/// to `hit.line + 10` safely covers the viewport for every slice from that
/// file. Used as the upper bound for `highlight_lines_until` so we skip
/// parsing megabytes of unreachable content.
const SEARCH_SLICE_HIGHLIGHT_TAIL: usize = 10;

/// Open every hit as a "search stack" tab — one read-only slice per match.
/// Always creates a fresh tab so repeated searches can be compared. The
/// total count is bounded only by the search engine's `MAX_RESULTS` cap;
/// slices from the same file share an `Arc<Vec<String>>` line buffer so
/// the extra cost of an unbounded stack is O(number of unique files), not
/// O(number of hits).
///
/// Highlighting runs asynchronously per unique file: the tab opens
/// immediately with unhighlighted (plain-text) slices, and one
/// [`Message::SearchStackHighlighted`] arrives per file when its windowed
/// highlight completes.
fn open_search_stack_tab(
    state: &mut State,
    query: &str,
    hits: Vec<widget::text_search::SearchHit>,
) -> Task<Message> {
    use std::collections::HashMap;

    let total = hits.len();

    // Build a base editor once per unique file. All slices for a file
    // clone this base; `EditorState.lines` is `Arc<Vec<String>>`, so the
    // line buffer is refcount-shared (O(1) per slice) rather than deep-
    // cloned. `max_hit_line` drives the windowed highlight's stop row.
    let mut base_editors: HashMap<std::path::PathBuf, widget::text_edit::EditorState> =
        HashMap::new();
    let mut max_hit_line: HashMap<std::path::PathBuf, usize> = HashMap::new();

    let mut slices: Vec<tab_bar::SearchSlice> = Vec::with_capacity(hits.len());
    for hit in hits {
        if !base_editors.contains_key(&hit.abs_path) {
            let Ok(content) = std::fs::read_to_string(&hit.abs_path) else {
                continue;
            };
            base_editors.insert(
                hit.abs_path.clone(),
                widget::text_edit::EditorState::new(&content),
            );
        }
        let base = &base_editors[&hit.abs_path];

        // Clone: shares `lines` Arc; per-slice fields below get their own
        // values so each match line's background can differ.
        let mut editor = base.clone();
        editor.line_backgrounds = vec![None; editor.lines.len()];
        if let Some(slot) = editor.line_backgrounds.get_mut(hit.line) {
            *slot = Some(widget::text_edit::LineBgKind::Match);
        }
        // Center the match line within the slice's viewport
        // (per_slice_h = 10 lines * 20px = 200px).
        let slice_height = 10.0 * 20.0;
        let target_y = hit.line as f32 * 20.0 + 4.0 - (slice_height / 2.0) + 10.0;
        editor.scroll_y = target_y.max(0.0);
        editor.cursor = widget::text_edit::Pos::new(hit.line, 0);

        max_hit_line
            .entry(hit.abs_path.clone())
            .and_modify(|v| *v = (*v).max(hit.line))
            .or_insert(hit.line);

        slices.push(tab_bar::SearchSlice {
            rel_path: hit.rel_path,
            abs_path: hit.abs_path,
            line: hit.line,
            editor,
        });
    }
    if slices.is_empty() {
        return Task::none();
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let tab_id = format!("search:{now}");
    let title = if total > slices.len() {
        format!("search: {query} ({}/{total})", slices.len())
    } else {
        format!("search: {query}")
    };

    ensure_active_area(&mut state.active_area);
    let highlighter = state.highlighter.clone();
    state
        .tabs
        .open_search_stack(tab_id.clone(), title, query.to_string(), slices);

    // Kick off one parallel highlight job per unique file. Each job emits a
    // `SearchStackHighlighted` message; the handler fans the spans out to
    // every slice sharing that `abs_path`. `lines` is the same `Arc` the
    // slices hold, so the blocking task reads the shared buffer without a
    // copy.
    let mut jobs: Vec<Task<Message>> = Vec::with_capacity(base_editors.len());
    for (abs_path, base) in base_editors {
        let last_line =
            max_hit_line.get(&abs_path).copied().unwrap_or(0) + SEARCH_SLICE_HIGHLIGHT_TAIL;
        let ext = abs_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("txt")
            .to_string();
        let lines = base.lines.clone();
        let highlighter_job = highlighter.clone();
        let tab_id_msg = tab_id.clone();
        let abs_path_msg = abs_path.clone();
        jobs.push(Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    let syntax = highlighter_job.find_syntax(&ext);
                    highlighter_job.highlight_lines_until(&lines, syntax, last_line)
                })
                .await
                .unwrap_or_default()
            },
            move |spans| Message::SearchStackHighlighted {
                tab_id: tab_id_msg.clone(),
                abs_path: abs_path_msg.clone(),
                spans: Arc::new(spans),
            },
        ));
    }
    Task::batch(jobs)
}

/// Apply an editor action targeted at one slice of the active SearchStack tab.
pub fn handle_search_slice_action(
    tabs: &mut tab_bar::TabState,
    idx: usize,
    action: widget::text_edit::EditorAction,
) {
    if let Some(tab) = tabs.active_tab_mut()
        && let tab_bar::TabView::SearchStack { slices, .. } = &mut tab.view
        && let Some(slice) = slices.get_mut(idx)
    {
        let _ = slice.editor.apply_action(action);
    }
}

/// Open the slice at `idx` of the active SearchStack as a new editable file
/// tab. Scrolls to the clicked match and highlights every other match from
/// the same file in the stack, so the full tab mirrors the stack view.
pub fn handle_open_search_slice(
    tabs: &mut tab_bar::TabState,
    idx: usize,
    highlighter: &highlight::SyntaxHighlighter,
) {
    let Some(tab) = tabs.active_tab() else {
        return;
    };
    let tab_bar::TabView::SearchStack { slices, .. } = &tab.view else {
        return;
    };
    let Some(slice) = slices.get(idx) else {
        return;
    };
    let rel = slice.rel_path.clone();
    let abs = slice.abs_path.clone();
    let line = slice.line;
    let match_lines: Vec<usize> = slices
        .iter()
        .filter(|s| s.rel_path == rel)
        .map(|s| s.line)
        .collect();
    let Ok(content) = std::fs::read_to_string(&abs) else {
        return;
    };
    let id = format!("file:{rel}");
    let title = std::path::Path::new(&rel)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| rel.clone());
    tabs.open_file(id.clone(), title, content, Some(abs));
    if let Some(tab) = tabs.file_tabs.iter_mut().find(|t| t.id == id)
        && let tab_bar::TabView::Editor { editor, .. } = &mut tab.view
    {
        rehighlight(editor, &id, highlighter);
        set_match_line_highlights(editor, &match_lines);
        let target_y = line as f32 * 20.0 - 300.0;
        editor.scroll_y = target_y.max(0.0);
        editor.cursor = widget::text_edit::Pos::new(line, 0);
    }
}

// ── Artifact tab helper ─────────────────────────────────────────────────────

/// Open a file as a text editor tab. Called from area update functions.
pub fn open_artifact_tab(
    tabs: &mut tab_bar::TabState,
    id: String,
    title: String,
    source: String,
    _artifact_id: &str,
    path: Option<std::path::PathBuf>,
    highlighter: &highlight::SyntaxHighlighter,
) {
    tabs.open_preview(id.clone(), title, source, path);
    if let Some(tab) = tabs.preview.as_mut()
        && tab.id == id
        && let tab_bar::TabView::Editor { editor, .. } = &mut tab.view
    {
        rehighlight(editor, &id, highlighter);
    }
}

// ── Agent helpers ───────────────────────────────────────────────────────────

fn flush_pending_text(session: &mut chat_store::ChatSession) {
    if !session.pending_text.is_empty() {
        let text = std::mem::take(&mut session.pending_text);
        session.messages.push(chat_store::ChatMessage {
            role: chat_store::Role::Assistant,
            content: vec![chat_store::ContentBlock::Text(text)],
            timestamp: String::new(),
        });
    }
}

/// Apply a title-summary result to the session identified by `key`, and —
/// if the session belongs to an exploration scope — also update the
/// exploration's display_name so the dashboard/list show the new title.
/// Re-reconciles the owning interaction's session display names and persists.
fn apply_session_title(state: &mut State, key: &str, title: &str) {
    let proj_root = state.project.project_root.clone();

    // Look up the session and mark it titled. Collect the info we need
    // before releasing the borrow.
    let Some((scope_key, scope_kind)) = ({
        let Some(ax) = state.agent_session_mut(key) else {
            return;
        };
        if ax.session.title.is_some() {
            return;
        }
        ax.session.title = Some(title.to_string());
        if let Err(e) = chat_store::save_session(&ax.session, proj_root.as_deref()) {
            tracing::error!(key, "failed to save chat session after title: {e}");
        }
        Some((ax.session.scope.clone(), ax.scope_kind))
    }) else {
        return;
    };

    // For explorations: the title also renames the exploration itself.
    if scope_kind == scope::ScopeKind::Exploration {
        if let Some(exp) = state
            .change
            .explorations
            .iter_mut()
            .find(|e| e.id == scope_key)
        {
            exp.display_name = title.to_string();
        }
        chat_store::save_explorations(
            &state.change.explorations,
            state.change.exploration_counter,
            proj_root.as_deref(),
        );
    }

    // Re-reconcile display names in the owning interaction so the new title
    // (or exploration display_name) propagates to the session dropdown.
    let label = state.change.scope_display_label(&scope_key);
    if let Some(ix) = state.interaction_mut(&scope_key) {
        interaction::reconcile_display_names(&mut ix.sessions, &label);
    }
}

// ── View ─────────────────────────────────────────────────────────────────────

/// Compose the shared three-column layout for any non-dashboard, non-settings
/// area: list (per-area) | content (global tabs) | optional interaction.
fn view_area_three_column(state: &State) -> Element<'_, Message> {
    let list: Element<'_, Message> = match state.active_area {
        Area::Change => area::change::view_list(
            &state.change,
            &state.project,
            &state.ideas,
            &state.tabs,
        )
        .map(Message::Change),
        Area::Caps => area::caps::view_list(&state.caps, &state.project, &state.tabs)
            .map(Message::Caps),
        Area::Codex => area::codex::view_list(&state.codex, &state.project, &state.tabs)
            .map(Message::Codex),
        Area::Ideas => area::ideas::view_list(&state.ideas, &state.tabs).map(Message::Ideas),
        _ => unreachable!("view_area_three_column called for area without three-column layout"),
    };

    let content = view_global_content(state);
    let scope = state.active_scope();
    let ix = scope.as_ref().and_then(|s| state.interactions.get(s));
    let controls = match scope {
        Some(scope::Scope::Change(_)) => interaction::SessionControls::Multi,
        _ => interaction::SessionControls::Single,
    };

    let divider = container(Space::new().height(Length::Fill))
        .width(1.0)
        .style(theme::divider);

    // Change area exploration mode: skip the content column when there are no
    // tabs open, so the empty state instructions are visible without an empty
    // editor consuming the middle of the screen.
    let is_exploration =
        state.active_area == Area::Change && state.change.is_exploration_selected();
    let has_tabs = state.tabs.preview.is_some() || !state.tabs.file_tabs.is_empty();

    let mut row_items = row![
        container(list)
            .width(theme::LIST_COLUMN_WIDTH)
            .height(Length::Fill)
            .style(theme::surface),
        divider,
    ];

    if !is_exploration || has_tabs {
        row_items = row_items.push(
            container(content)
                .width(Length::Fill)
                .height(Length::Fill),
        );
    }

    let visible = ix.is_some_and(|i| i.visible);
    let width = ix.map_or(theme::INTERACTION_COLUMN_WIDTH, |i| i.width);

    if !is_exploration || has_tabs {
        let toggle = widget::interaction_toggle::view(visible, width, |m| {
            Message::Interaction(interaction::Msg::Handle(m))
        });
        row_items = row_items.push(toggle);
    }

    if let Some(ix) = ix
        && ix.visible
    {
        let interaction_col =
            interaction::view_column(ix, Message::Interaction, controls);
        let col = container(interaction_col)
            .height(Length::Fill)
            .style(theme::surface);
        let col = if !is_exploration || has_tabs {
            col.width(ix.width)
        } else {
            col.width(Length::Fill)
        };
        row_items = row_items.push(col);
    }

    row_items.height(Length::Fill).into()
}

/// Render the shared content column: tab bar + (optional) area-specific
/// toolbar + tab content + (Change-only) error panel.
fn view_global_content(state: &State) -> Element<'_, Message> {
    let bar = tab_bar::view_bar(&state.tabs, Message::TabSelect, Message::TabClose);
    let body = tab_bar::view_content(&state.tabs).map(Message::TabContent);

    let mut col = column![bar];

    // Idea pinned-tab toolbar (Explore / Open Change / Archive / Delete).
    if state.active_area == Area::Ideas
        && let Some(toolbar) = area::ideas::view_pinned_toolbar(&state.ideas, &state.tabs)
    {
        col = col.push(toolbar.map(Message::Ideas));
    }

    col = col.push(body);

    // Change area: error panel for the active artifact.
    if state.active_area == Area::Change
        && let Some(errors) = area::change::error_panel_for(&state.change, &state.project, &state.tabs)
    {
        let divider = container(Space::new().width(Length::Fill))
            .height(1.0)
            .style(theme::divider);
        let mut error_list = column![].spacing(theme::SPACING_XS);
        for err in errors {
            error_list = error_list.push(
                iced::widget::text(err.as_str())
                    .size(theme::font_md())
                    .color(theme::error()),
            );
        }
        let panel = container(
            column![
                iced::widget::text("Errors")
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

    col.height(Length::Fill).into()
}

fn view(state: &State) -> Element<'_, Message> {
    let next_mode = match theme::mode() {
        theme::ColorMode::Dark => theme::ColorMode::Light,
        theme::ColorMode::Light => theme::ColorMode::Dark,
    };
    let sidebar = widget::sidebar::view(
        &state.active_area,
        state.project.project_root.is_some(),
        Message::AreaSelected,
        Message::Refresh,
        Message::ThemeChanged(next_mode),
    );

    let area_content: Element<'_, Message> = match state.active_area {
        Area::Dashboard => area::dashboard::view(
            &state.dashboard,
            &state.project,
            &state.change.explorations,
            &state.config.projects.recent,
        )
        .map(Message::Dashboard),
        Area::Settings => {
            area::settings::view(&state.settings, &state.config).map(Message::Settings)
        }
        _ => view_area_three_column(state),
    };

    let segments = match state.active_area {
        Area::Dashboard => area::dashboard::breadcrumbs(),
        Area::Ideas => area::ideas::breadcrumbs(&state.ideas),
        Area::Change => area::change::breadcrumbs(&state.change, &state.project, &state.tabs),
        Area::Caps => area::caps::breadcrumbs(&state.tabs),
        Area::Codex => area::codex::breadcrumbs(&state.tabs),
        Area::Settings => area::settings::breadcrumbs(),
    };
    let project_label = state
        .project
        .project_root
        .as_ref()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()));
    let status_bar = widget::status_bar::view(segments, project_label);
    let status_divider = container(Space::new().width(Length::Fill))
        .height(1.0)
        .style(theme::divider);
    let area_with_status = column![
        container(area_content).height(Length::Fill),
        status_divider,
        status_bar,
    ]
    .height(Length::Fill);

    let sidebar_divider = container(Space::new().height(Length::Fill))
        .width(1.0)
        .style(theme::divider);
    let top_divider = container(Space::new().width(Length::Fill))
        .height(1.0)
        .style(theme::divider);
    let main_view = column![
        top_divider,
        row![sidebar, sidebar_divider, area_with_status].height(Length::Fill),
    ]
    .height(Length::Fill);

    if state.project_picker.visible {
        let overlay = widget::project_picker::view(
            &state.project_picker,
            &state.config.projects.recent,
        )
        .map(Message::ProjectPicker);
        stack![main_view, overlay].into()
    } else if state.file_finder.visible {
        let overlay = widget::file_finder::view(&state.file_finder).map(Message::FileFinder);
        stack![main_view, overlay].into()
    } else if state.text_search.visible {
        let overlay = widget::text_search::view(&state.text_search).map(Message::TextSearch);
        stack![main_view, overlay].into()
    } else if state.quick_idea.visible {
        let overlay = widget::quick_idea::view(&state.quick_idea).map(Message::QuickIdea);
        stack![main_view, overlay].into()
    } else {
        main_view.into()
    }
}

// ── Subscription ────────────────────────────────────────────────────────────

fn subscription(state: &State) -> Subscription<Message> {
    let mut subs = vec![];

    // File watcher: active when project root is known.
    if let Some(root) = state.project.project_root.as_ref() {
        subs.push(
            watcher::watch_subscription(root.clone(), state.project.duckspec_root.clone())
                .map(Message::FileChanged),
        );
    }

    // Per-terminal PTY subscriptions. Keyed by the stable `instance_id` and
    // the per-tab `terminal.id` so each tab's shell survives scope renames
    // (e.g. exploration→change promotion) and tab reorders.
    let pty_cwd = state.project.project_root.clone();
    let push_pty = |ix: &interaction::InteractionState, subs: &mut Vec<Subscription<Message>>| {
        for tt in &ix.terminals {
            let key = format!("pty:ix:{}/term:{}", ix.instance_id, tt.id);
            subs.push(widget::terminal::pty_subscription(key, pty_cwd.clone()).map(tagged_pty));
        }
    };
    for ix in state.interactions.values() {
        push_pty(ix, &mut subs);
    }

    // Per-session agent subscriptions. Key format: `agent:ix:<instance_id>/<session_id>`.
    // Like PTYs, keyed by `instance_id` so in-flight agent streams survive renames.
    if let Some(root) = state.project.project_root.as_ref() {
        let push_scope = |ix: &interaction::InteractionState,
                          subs: &mut Vec<Subscription<Message>>| {
            for session in &ix.sessions {
                let key = format!("agent:ix:{}/{}", ix.instance_id, session.session.id);
                subs.push(agent::agent_subscription(key, root.clone()).map(tagged_agent));
            }
        };
        for ix in state.interactions.values() {
            push_scope(ix, &mut subs);
        }
    }

    // Global keyboard events.
    subs.push(event::listen_raw(handle_key_event));

    // Poll system dark/light mode.
    subs.push(theme_subscription());

    // Animation tick for the streaming indicator. Only subscribed when at
    // least one session is actively streaming, so idle chats don't wake
    // the render loop. Uses iced's built-in `time::every` so the timer runs
    // on iced's tokio runtime — the earlier handcrafted `tokio::time::sleep`
    // stream panicked silently under the default thread-pool backend.
    if any_session_streaming(state) {
        subs.push(
            iced::time::every(std::time::Duration::from_millis(
                widget::streaming_indicator::TICK_MS,
            ))
            .map(|_instant| Message::StreamTick),
        );
    }

    Subscription::batch(subs)
}

/// True if any session across all interaction panels is actively streaming.
fn any_session_streaming(state: &State) -> bool {
    state
        .interactions
        .values()
        .any(|ix| ix.sessions.iter().any(|s| s.session.is_streaming))
}

fn theme_subscription() -> Subscription<Message> {
    Subscription::run(theme_detect_stream).map(Message::ThemeChanged)
}

fn theme_detect_stream() -> impl iced::futures::Stream<Item = theme::ColorMode> {
    use iced::futures::stream::{self, StreamExt};
    use std::sync::atomic::{AtomicU8, Ordering};
    static LAST: AtomicU8 = AtomicU8::new(u8::MAX);
    stream::unfold((), |()| async {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        let current = theme::detect_mode();
        Some((current, ()))
    })
    .filter(move |current| {
        let cur_val = *current as u8;
        let prev_val = LAST.swap(cur_val, Ordering::Relaxed);
        async move { prev_val != cur_val }
    })
}

// Non-capturing mapper functions for Subscription::map.
// The key embedded in the tuple carries the routing info.
fn tagged_pty((key, e): (String, widget::terminal::PtyEvent)) -> Message {
    // Key shape: `pty:ix:{instance_id}/term:{terminal_id}`.
    let rest = key.strip_prefix("pty:ix:").unwrap_or(&key);
    let (ix_str, term_str) = rest.split_once("/term:").unwrap_or((rest, ""));
    let ix_id = ix_str.parse::<u64>().unwrap_or(0);
    let terminal_id = term_str.parse::<u64>().unwrap_or(0);
    Message::PtyEvent(ix_id, terminal_id, e)
}
fn tagged_agent((key, e): (String, agent::AgentEvent)) -> Message {
    // Strip the `agent:ix:` prefix; the remainder is `<instance_id>/<session_id>`.
    let routing_key = key.strip_prefix("agent:ix:").unwrap_or(&key).to_string();
    Message::AgentEvent(routing_key, e)
}

/// Launch another duckboard process detached from this one. Used by
/// Cmd+Shift+N to give the user a second window — Iced 0.14's single-window
/// model means a new window is necessarily a new process.
fn spawn_new_instance() {
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("spawn_new_instance: current_exe failed: {e}");
            return;
        }
    };
    match std::process::Command::new(&exe).spawn() {
        Ok(child) => tracing::info!(pid = child.id(), exe = %exe.display(), "spawned new duckboard instance"),
        Err(e) => tracing::warn!(exe = %exe.display(), "spawn_new_instance: spawn failed: {e}"),
    }
}

fn main() -> iced::Result {
    // Must run before any threads are spawned (tracing, iced runtime, etc.).
    // When launched from Finder, launchd gives the .app bundle a stripped
    // PATH that misses every user-level install dir — any Command::new spawn
    // in the app would fail with ENOENT.
    path_env::augment();

    // Kick off a background harvest of the user's login-shell env so
    // subprocesses we later spawn (claude and anything it runs) see the
    // same tool-manager activation (mise, asdf, nix, rustup, …) the user
    // gets in their terminal. Non-blocking — just starts the thread.
    duckchat::shell_env::init();

    tracing_subscriber::fmt::init();

    // Detect system dark/light mode before creating the window.
    theme::set_mode(theme::detect_mode());
    tracing::info!(mode = ?theme::mode(), "duckboard starting");

    iced::application(State::new, update, view)
        .subscription(subscription)
        .title("duckboard")
        .theme(theme_fn)
        .window_size((1200.0, 800.0))
        .run()
}

fn theme_fn(_state: &State) -> iced::Theme {
    theme::app_theme()
}

fn handle_key_event(
    event: Event,
    status: event::Status,
    _window: iced::window::Id,
) -> Option<Message> {
    match event {
        Event::Keyboard(keyboard::Event::ModifiersChanged(mods)) => {
            // Mirror modifier state into a process-wide cell so canvas widgets
            // (terminal, etc.) can react to cmd-held mouse moves and clicks.
            widget::terminal::set_current_modifiers(mods);
            None
        }
        Event::Keyboard(keyboard::Event::KeyPressed {
            key,
            modifiers,
            text,
            ..
        }) => {
            widget::terminal::set_current_modifiers(modifiers);
            // Skip events already consumed by a focused widget (e.g. Enter typed
            // into the content editor). Otherwise the chat column would also
            // react to them. Escape is exempt: iced's `text_input` captures it to
            // clear focus, so without the exemption the file finder would need
            // two Escape presses to close.
            let is_escape = matches!(&key, keyboard::Key::Named(keyboard::key::Named::Escape));
            if !is_escape && matches!(status, event::Status::Captured) {
                return None;
            }
            Some(Message::KeyPress(
                key,
                modifiers,
                text.map(|s| s.to_string()),
            ))
        }
        _ => None,
    }
}
