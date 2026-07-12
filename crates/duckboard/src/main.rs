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
mod default_prompts;
mod meta_card;
mod fast_response;
pub mod highlight;
mod idea_format;
mod idea_store;
mod keybinds;
mod path_env;
mod path_link;
mod scope;
mod theme;
mod title_hints;
mod vcs;
mod watcher;
mod widget;

use area::Area;
use area::interaction::{self, ActiveTab};
use data::ProjectData;
use keybinds::FocusedColumn;
use widget::tab_bar;

// ── Constants for routing keys ──────────────────────────────────────────────

const KEY_CAPS: &str = "caps";
const KEY_CODEX: &str = "codex";

// ── State ────────────────────────────────────────────────────────────────────

pub(crate) struct State {
    pub(crate) active_area: Area,
    pub(crate) project: ProjectData,
    config: config::Config,
    dashboard: area::dashboard::State,
    ideas: area::ideas::State,
    pub(crate) change: area::change::State,
    caps: area::caps::State,
    codex: area::codex::State,
    settings: area::settings::State,
    file_finder: widget::file_finder::FileFinderState,
    text_search: widget::text_search::TextSearchState,
    project_picker: widget::project_picker::ProjectPickerState,
    quick_idea: widget::quick_idea::QuickIdeaState,
    new_file: widget::new_file::NewFileState,
    find_modal: widget::find::FindModalState,
    /// Active local-find state per target. Editor finds key by tab id;
    /// chat finds key by `(instance_id, session_id)`. Survives tab/session
    /// switches; cleared by Esc / click-away on the modal, the toolbar's
    /// cancel, or a fresh cmd-f opening the modal again.
    find_states: HashMap<widget::find::FindTarget, widget::find::FindState>,
    /// Most-recently-focused content column. Drives the cmd-f scope rule:
    /// `Chat` → target the active session, `Content` → target the active
    /// editor tab. `None` → cmd-f is a no-op.
    pub(crate) focused_column: Option<FocusedColumn>,
    /// Set by `jump_to_current` when find scrolls the chat to a match.
    /// Read by `update_with_scroll_preservation` to skip the post-update
    /// chat-scroll replay (which would otherwise restore the pre-find
    /// position and visually undo the jump). Reset every wrapper tick.
    chat_scroll_overridden: bool,
    /// Shared via `Arc` so background tasks (e.g. search-stack highlighting)
    /// can hold a handle without blocking the UI on syntax-set ownership.
    highlighter: Arc<highlight::SyntaxHighlighter>,
    /// Single tab stack shared across Change/Caps/Codex/Ideas. The active
    /// area drives the `preview` (pinned) slot via its list selection;
    /// `file_tabs` (closable) persist across area switches.
    pub(crate) tabs: tab_bar::TabState,
    /// Logical index of the dirty file tab whose close × has been clicked
    /// once and is now armed — a second `TabClose` for the same index
    /// commits, discarding unsaved edits. Cleared by `TabSelect`, by any
    /// `TabClose` of a different index (or success), and by the next
    /// keypress in the keyboard subscription.
    armed_tab_close: Option<usize>,
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
    pub(crate) interactions: HashMap<scope::Scope, interaction::InteractionState>,
    /// Logical window width for equal content/chat free-space split.
    /// Seeded from the default window size; updated on resize.
    window_width: f32,
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
        interactions.insert(
            scope::Scope::Codex,
            interaction::InteractionState::default(),
        );
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
            new_file: widget::new_file::NewFileState::default(),
            find_modal: widget::find::FindModalState::default(),
            find_states: HashMap::new(),
            focused_column: None,
            chat_scroll_overridden: false,
            highlighter: Arc::new(highlight::SyntaxHighlighter::new()),
            tabs: tab_bar::TabState::default(),
            armed_tab_close: None,
            cached_previews: HashMap::new(),
            cached_active: HashMap::new(),
            interactions,
            window_width: theme::DEFAULT_WINDOW_WIDTH,
        }
    }

    /// Switch to the project rooted at `path`. Rebuilds subordinate area
    /// state so stale tabs / interactions from the previous project are
    /// discarded, then refreshes audit and recents.
    fn open_project(&mut self, path: PathBuf) {
        // Strip trailing separators / canonicalize so recents, data-dir hashes,
        // and grok session cwd keys stay stable across picker vs recents open.
        let path = duckchat::normalize_cwd(&path);
        tracing::info!(path = %path.display(), "opening project");
        self.project = ProjectData::open(&path);
        // Mirror the root into the path_link global so editors and terminals
        // can resolve relative path references at hover time.
        path_link::set_project_root(self.project.project_root.clone());
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
        // At project open there are no open tabs or selection to follow, so the
        // reported relocations are discarded.
        let _ = idea_store::reconcile(&mut self.ideas.ideas, &self.project);
        // Drop interactions / tabs from the prior project; reseed singletons.
        self.tabs = tab_bar::TabState::default();
        self.armed_tab_close = None;
        self.cached_previews.clear();
        self.cached_active.clear();
        self.interactions.clear();
        self.interactions
            .insert(scope::Scope::Caps, interaction::InteractionState::default());
        self.interactions.insert(
            scope::Scope::Codex,
            interaction::InteractionState::default(),
        );
        self.project.revalidate();
        self.active_area = Area::Dashboard;

        self.config.projects.touch(&path);
        if let Err(e) = config::save(&self.config) {
            tracing::warn!("failed to persist recent projects: {e}");
        }
    }

    /// Drop `path` from the recent-projects list and persist the config.
    /// Reversible: reopening the project re-touches it back to the top.
    fn forget_recent(&mut self, path: &Path) {
        self.config.projects.recent.retain(|p| p != path);
        self.dashboard.hovered_recent = None;
        self.dashboard.armed_delete_recent = None;
        self.project_picker.hovered_recent = None;
        self.project_picker.armed_delete_recent = None;
        if let Err(e) = config::save(&self.config) {
            tracing::warn!("failed to persist recent projects: {e}");
        }
    }

    /// Drop `path` from recents AND wipe its on-disk data directory
    /// (chats, ideas, explorations). Irrecoverable.
    fn delete_recent_data(&mut self, path: &Path) {
        chat_store::delete_project_data(path);
        self.forget_recent(path);
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
    pub(crate) fn active_scope(&self) -> Option<scope::Scope> {
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
    pub(crate) fn active_interaction_key(&self) -> Option<String> {
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
    /// First click on the × of a dirty file tab. Arms `armed_tab_close` so
    /// the next matching `TabClose` commits, discarding unsaved edits.
    TabArmClose(usize),
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
    // New-file modal — create-or-open by typed path (cmd+n when content focused).
    NewFile(widget::new_file::Msg),
    // Local find — cmd-f within the focused editor or chat session.
    /// Open the find modal targeting the focused column. No payload — the
    /// handler reads `state.focused_column` and builds the snapshot.
    FindOpen,
    Find(widget::find::Msg),
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
    // Result of the reply-suggestion oneshot after a turn. `prompts_gen` must
    // match the session's `default_prompts_gen` or the result is dropped.
    DefaultPromptsReady {
        key: String,
        prompts_gen: u64,
        result: Result<Vec<String>, String>,
    },
    // Settings
    Settings(area::settings::Message),
    // System theme changed
    ThemeChanged(theme::ColorMode),
    /// App-start model catalog refresh finished; re-read pickers / oneshot resolve.
    ModelCatalogReady,
    // Animation tick for the streaming indicator; only fires while a session
    // is streaming (see `subscription`).
    StreamTick,
    // ~60fps tick that advances every terminal whose drag is auto-scrolling
    // past an edge. Only fires while a drag holds the pointer past an edge
    // (see `subscription` / `any_terminal_autoscrolling`).
    TerminalAutoscrollTick,
    // Coalesced ~1s tick while a session streams; persists dirty sessions so
    // mid-turn loss is bounded to roughly one interval (see `subscription`).
    FlushTick,
    // The window received a close request. We persist every session before
    // letting the window actually close (see `main`'s `exit_on_close_request`).
    WindowCloseRequested(iced::window::Id),
    /// Logical window size changed — rebalance uncustomized interaction widths.
    WindowResized(iced::Size),
}

// ── Update ───────────────────────────────────────────────────────────────────

/// Stamp every chat session with the current project's default model
/// (`ModelRef`). Cheap (a handful of sessions) and run once per update tick so
/// a freshly-created session or a default just changed in Settings is reflected
/// before the next send. `Config` and `project_root` live here on the global
/// state; the interaction layer can't reach them, so it reads the stamped
/// value off `AgentSession::project_model_default` instead.
fn refresh_model_defaults(state: &mut State) {
    let default = state
        .project
        .project_root
        .as_deref()
        .and_then(|root| state.config.project_model_default(root));
    for ix in state.interactions.values_mut() {
        for ax in ix.sessions.iter_mut() {
            ax.project_model_default = default.clone();
        }
    }
}

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
    update_focused_column(state, &message);
    refresh_model_defaults(state);
    // Cmd-clicked path references can surface from any editor or terminal
    // route, but opening them needs `State` (tabs, file finder, project
    // root) that the per-area handlers don't have — intercept centrally.
    if let Some((path, line)) = extract_open_path(&message) {
        return open_path_reference(state, &path, line);
    }
    match message {
        Message::AreaSelected(area) => {
            switch_area(state, area);
            if area == Area::Settings {
                area::settings::update(
                    &mut state.settings,
                    &mut state.config,
                    state.project.project_root.as_deref(),
                    area::settings::Message::LoadFonts,
                );
            }
            return restore_chat_scroll(state);
        }
        Message::ModelCatalogReady => {
            // Catalog was filled on a background task; this message forces a
            // re-render and subscription rebuild so model/oneshot pickers and
            // worker oneshot preferred ids re-resolve from the live catalog.
            tracing::info!(
                models = agent::available_models().len(),
                "model catalog ready"
            );
            return Task::none();
        }
        Message::Refresh => {
            let outcome = reload_and_reconcile(state);
            let mut tasks: Vec<Task<Message>> = Vec::new();
            refresh_open_tabs(state, &mut tasks);
            refresh_changed_files(state);
            if state
                .change
                .expanded_sections
                .contains(area::change::FILES_SECTION)
            {
                refresh_project_files(state);
            }
            state.project.revalidate();
            tracing::info!("project reloaded");
            // Bound exploration→change promotion remounts the chat tree —
            // re-focus so the user can keep typing without a re-click.
            if outcome.promoted {
                tasks.push(focus_chat_input());
            }
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
                            let line = state.file_finder.pending_line.take();
                            task = open_path_in_tab(state, abs, line);
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
                    return iced::widget::operation::focus(widget::text_search::SEARCH_INPUT_ID);
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
                    let refocus =
                        iced::widget::operation::focus(widget::text_search::SEARCH_INPUT_ID);
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
                Msg::HoverRecent(path) => {
                    state.project_picker.hovered_recent = Some(path);
                }
                Msg::UnhoverRecent(path) => {
                    if state.project_picker.hovered_recent.as_deref() == Some(path.as_path()) {
                        state.project_picker.hovered_recent = None;
                    }
                    if state.project_picker.armed_delete_recent.as_deref() == Some(path.as_path()) {
                        state.project_picker.armed_delete_recent = None;
                    }
                }
                Msg::ForgetRecent(path) => {
                    state.forget_recent(&path);
                }
                Msg::ArmDeleteRecent(path) => {
                    state.project_picker.armed_delete_recent = Some(path);
                }
                Msg::DeleteRecentData(path) => {
                    state.delete_recent_data(&path);
                }
            }
        }
        Message::FindOpen => {
            return open_find_modal(state);
        }
        Message::Find(msg) => {
            return handle_find_msg(state, msg);
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
                        state.quick_idea.load_selected(state.highlighter.as_ref());
                        return iced::widget::operation::focus(widget::quick_idea::INPUT_ID);
                    }
                    let payload = widget::quick_idea::build_save_payload(&state.quick_idea);
                    if !widget::quick_idea::body_is_savable(&payload.body) {
                        return Task::none();
                    }
                    let project_root = state.project.project_root.clone();
                    let duckspec_root = state.project.duckspec_root.clone();
                    save_quick_idea(
                        state,
                        payload,
                        project_root.as_deref(),
                        duckspec_root.as_deref(),
                    );
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
        Message::NewFile(msg) => {
            use widget::new_file::Msg;
            match msg {
                Msg::OpenAt(starting) => {
                    let Some(root) = state.project.project_root.clone() else {
                        return Task::none();
                    };
                    state.new_file.open_at(&root, starting);
                    for ix in state.interactions.values_mut() {
                        ix.terminal_focused = false;
                    }
                    return Task::batch([
                        iced::widget::operation::focus(widget::new_file::INPUT_ID),
                        iced::widget::operation::move_cursor_to_end(widget::new_file::INPUT_ID),
                    ]);
                }
                Msg::Close => {
                    state.new_file.close();
                }
                Msg::QueryChanged(q) => {
                    if state.new_file.handle_input(q) {
                        return iced::widget::operation::move_cursor_to_end(
                            widget::new_file::INPUT_ID,
                        );
                    }
                }
                Msg::SelectNext => state.new_file.select_next(),
                Msg::SelectPrev => state.new_file.select_prev(),
                Msg::TabComplete => {
                    state.new_file.tab_complete();
                    return iced::widget::operation::move_cursor_to_end(widget::new_file::INPUT_ID);
                }
                Msg::Confirm => {
                    let action = state.new_file.confirm_action();
                    state.new_file.close();
                    if let Some(action) = action {
                        return confirm_new_file(state, action);
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
                            refresh_file_tabs_for_path(state, root, path, &mut highlight_tasks);
                            refresh_diff_tabs_for_path(state, root, path, &mut highlight_tasks);
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

            if tree_changed {
                let outcome = reload_and_reconcile(state);
                if outcome.archived {
                    // Tab IDs were rewritten to new archive paths; re-read
                    // their content from disk so editors reflect the moved files.
                    refresh_open_tabs(state, &mut highlight_tasks);
                }
                if outcome.promoted {
                    // Bound promotion remounts chat under the change scope.
                    highlight_tasks.push(focus_chat_input());
                }
            }

            if vcs_state_changed && let Some(root) = project_root.as_deref() {
                refresh_all_diff_tabs(state, root, &mut highlight_tasks);
            }

            refresh_changed_files(state);
            if state
                .change
                .expanded_sections
                .contains(area::change::FILES_SECTION)
            {
                refresh_project_files(state);
            }

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
                        state.config.chat.agent_input_hints,
                        state.window_width,
                        );
                    return restore_chat_scroll(state);
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
                        state.config.chat.agent_input_hints,
                        state.window_width,
                        );
                    return Task::batch([restore_chat_scroll(state), focus_chat_input()]);
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
                        state.config.chat.agent_input_hints,
                        state.window_width,
                        );
                    return restore_chat_scroll(state);
                }
                area::dashboard::Message::HoverRecent(path) => {
                    state.dashboard.hovered_recent = Some(path.clone());
                }
                area::dashboard::Message::UnhoverRecent(path) => {
                    if state.dashboard.hovered_recent.as_deref() == Some(path.as_path()) {
                        state.dashboard.hovered_recent = None;
                    }
                    // Hover off the row → disarm the destructive button.
                    if state.dashboard.armed_delete_recent.as_deref() == Some(path.as_path()) {
                        state.dashboard.armed_delete_recent = None;
                    }
                }
                area::dashboard::Message::ForgetRecent(path) => {
                    state.forget_recent(path);
                }
                area::dashboard::Message::ArmDeleteRecent(path) => {
                    state.dashboard.armed_delete_recent = Some(path.clone());
                }
                area::dashboard::Message::DeleteRecentData(path) => {
                    state.delete_recent_data(path);
                }
            }
        }
        Message::Change(msg) => {
            match msg {
                area::change::Message::SelectChangedFile(path) => {
                    return open_diff_preview(state, Area::Change, &path);
                }
                area::change::Message::SelectExplorerFile(id) => {
                    // Explorer rows open the working-tree file (a `file:`
                    // tab), unlike changed-files rows which open a diff —
                    // the section clicked expresses the intent.
                    if let Some(root) = state.project.project_root.clone() {
                        let rel = id.strip_prefix("file:").unwrap_or(&id);
                        return open_path_in_tab(state, root.join(rel), None);
                    }
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
                            state.config.chat.agent_input_hints,
                        state.window_width,
                            );
                        return restore_chat_scroll(state);
                    }
                }
                area::change::Message::AddFile => {
                    // `+` header button always opens at project root, regardless
                    // of what's currently focused — the header is about adding
                    // a file to the project as a whole, not next to whatever
                    // tab happens to be active.
                    return update(
                        state,
                        Message::NewFile(widget::new_file::Msg::OpenAt(String::new())),
                    );
                }
                msg => {
                    let toggled_files = matches!(
                        &msg,
                        area::change::Message::ToggleSection(id)
                            if id == area::change::FILES_SECTION
                    );
                    let needs_focus = matches!(msg, area::change::Message::AddExploration)
                        || is_chat_focus_msg(extract_change_interaction_msg(&msg));
                    area::change::update(
                        &mut state.change,
                        &mut state.tabs,
                        &mut state.interactions,
                        msg,
                        &state.project,
                        &state.highlighter,
                        state.config.chat.agent_input_hints,
                        state.window_width,
                        );
                    if needs_focus {
                        return focus_chat_input();
                    }
                    if toggled_files
                        && state
                            .change
                            .expanded_sections
                            .contains(area::change::FILES_SECTION)
                    {
                        // Walk the tree the moment the section opens, and
                        // reveal whichever file tab is already active.
                        refresh_project_files(state);
                        return reveal_active_file_in_explorer(state);
                    }
                }
            }
        }
        Message::Caps(msg) => {
            let needs_focus = is_chat_focus_msg(extract_caps_interaction_msg(&msg));
            let ix = state.interactions.entry(scope::Scope::Caps).or_default();
            area::caps::update(
                &mut state.caps,
                &mut state.tabs,
                ix,
                msg,
                &state.project,
                &state.highlighter,
                state.config.chat.agent_input_hints,
                        state.window_width,
                );
            if needs_focus {
                return focus_chat_input();
            }
        }
        Message::Codex(msg) => {
            let needs_focus = is_chat_focus_msg(extract_codex_interaction_msg(&msg));
            let ix = state.interactions.entry(scope::Scope::Codex).or_default();
            area::codex::update(
                &mut state.codex,
                &mut state.tabs,
                ix,
                msg,
                &state.project,
                &state.highlighter,
                state.config.chat.agent_input_hints,
                        state.window_width,
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
                if let Some(idea) = state
                    .ideas
                    .ideas
                    .iter_mut()
                    .find(|i| i.abs_path == old_path)
                {
                    idea.frontmatter.exploration = Some(exp_id.clone());
                    idea.state = idea_store::IdeaState::Exploration;
                    let body = idea_store::read_body(&idea.abs_path).unwrap_or_default();
                    if let Err(e) =
                        idea_store::save_idea(idea, &body, state.project.project_root.as_deref())
                    {
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
                    state.config.chat.agent_input_hints,
                        state.window_width,
                    );
                // SelectIdea spawns the exploration session with
                // empty chrome; refresh so the chat input renders lifecycle
                // chrome (mirrors ideas.rs).
                let dirty = !state.change.changed_files.is_empty();
                area::change::refresh_fast_response(
                    &mut state.interactions,
                    &state.project,
                    state.config.chat.agent_input_hints,
                    dirty,
                );
                return focus_chat_input();
            }
            if let area::ideas::Message::OpenChange(ref change_name) = msg {
                // Ideas store the unprefixed change name in frontmatter, but
                // archived changes live under `YYYY-MM-DD-NN-<name>`. Resolve
                // to the canonical folder name so SelectChange can find it.
                let base = change_name.clone();
                let canonical = state
                    .project
                    .active_changes
                    .iter()
                    .find(|c| c.name == base)
                    .map(|c| c.name.clone())
                    .or_else(|| {
                        state
                            .project
                            .archived_changes
                            .iter()
                            .find(|c| {
                                c.name == base
                                    || crate::data::strip_archive_prefix(&c.name)
                                        == Some(base.as_str())
                            })
                            .map(|c| c.name.clone())
                    })
                    .unwrap_or(base);
                switch_area(state, Area::Change);
                area::change::update(
                    &mut state.change,
                    &mut state.tabs,
                    &mut state.interactions,
                    area::change::Message::SelectChange(canonical),
                    &state.project,
                    &state.highlighter,
                    state.config.chat.agent_input_hints,
                        state.window_width,
                    );
                return restore_chat_scroll(state);
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
            let focus_idea_editor = matches!(msg, area::ideas::Message::AddIdea);
            area::ideas::update(
                &mut state.ideas,
                &mut state.tabs,
                &mut state.interactions,
                msg,
                &state.project,
                &state.highlighter,
                state.config.chat.agent_input_hints,
                        state.window_width,
                );
            if focus_tag_input {
                return iced::widget::operation::focus(area::ideas::TAG_INPUT_ID);
            }
            if focus_idea_editor {
                return iced::widget::operation::focus(area::ideas::EDITOR_ID);
            }
            if needs_focus {
                return focus_chat_input();
            }
        }
        Message::Settings(msg) => {
            area::settings::update(
                &mut state.settings,
                &mut state.config,
                state.project.project_root.as_deref(),
                msg,
            );
            theme::set_fonts(&state.config);
        }
        Message::TabSelect(idx) => {
            state.armed_tab_close = None;
            state.tabs.select(idx);
        }
        Message::TabClose(idx) => {
            state.armed_tab_close = None;
            state.tabs.close(idx);
        }
        Message::TabArmClose(idx) => {
            state.armed_tab_close = Some(idx);
        }
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
            // Suppress tentative refresh for in-flight drags: the chip
            // appearing in the chat panel mid-drag reflows layout and can
            // shift the chat content under the user's cursor. `DragEnd`
            // (published by the editor on mouse release) is the signal
            // that selection is stable and the chip can land safely.
            let should_sync = !matches!(&action, widget::text_edit::EditorAction::Drag(_));
            let task = handle_editor_action(
                &mut state.tabs,
                state.active_area,
                action,
                state.highlighter.clone(),
            );
            if should_sync {
                sync_tentative_from_active_tab(state);
            }
            return task;
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
            // `(handle, scope_key, scope_kind, target_msg, command_hint_source, idea_description)`.
            // `command_hint_source` is the user-message-form text of the most
            // recent slash command — fed to `title_hints::build_hint` even
            // when the target message itself carries no command. Title summary
            // runs through the chat's `AgentHandle` oneshot path (no cold
            // provider construction).
            type TitleTaskInput = (
                duckchat::AgentHandle,
                String,
                scope::ScopeKind,
                String,
                Option<String>,
                Option<String>,
            );
            let mut title_task_input: Option<TitleTaskInput> = None;
            // `(handle, assistant, user, gen)` — freeform oneshot, no heuristic/cmds.
            type ReplyTaskInput = (
                duckchat::AgentHandle,
                String,
                Option<String>,
                u64,
            );
            let mut reply_task_input: Option<ReplyTaskInput> = None;
            // Staged `(folder-slug, exploration-id)` from a `ds create change`
            // tool call, committed to `pending_bindings` once the `ax` borrow
            // below is released.
            let mut staged_binding: Option<(String, String)> = None;
            // SessionNotFound (or stringified equivalent): clear the dead id and
            // re-dispatch after this block so we can borrow `highlighter`.
            let mut recover_lost_session = false;
            // Copy before mutably borrowing the session (config lives on `state`).
            let agent_input_hints = state.config.chat.agent_input_hints;
            // Structural / kind-switch events force an immediate chat UI
            // materialize; pure content deltas only set `chat_ui_dirty` and
            // wait for StreamTick (see chat/stream-ui).
            let mut force_materialize = false;
            {
                let Some(ax) = state.agent_session_mut(&key) else {
                    return Task::none();
                };
                let streaming_before = ax.session.is_streaming;
                match evt {
                    AgentEvent::Ready(handle) => {
                        // Seed the worker with a persisted session id so the next
                        // prompt resumes that conversation — but only when the id
                        // belongs to this turn's harness. After a harness switch
                        // the stored id is foreign (a Claude id can't `session/load`
                        // under grok), so we leave the worker to start fresh.
                        if let Some(sid) = ax.resumable_session_id().map(str::to_string) {
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
                        let tripped_before = ax.session.answer_thrash_tripped;
                        let kind_switch =
                            interaction::apply_answer_content_delta(&mut ax.session, &text);
                        if ax.session.answer_thrash_tripped && !tripped_before {
                            // Budget crossed: keep last draft, notice, cancel heat.
                            // Same priming cleanup as CancelPressed so TurnComplete
                            // cannot dispatch a staged follow-up after thrash.
                            interaction::on_answer_thrash_trip(&mut ax.session);
                            if let Some(handle) = &ax.agent_handle {
                                handle.cancel();
                            }
                            interaction::clear_priming_followup(ax);
                            force_materialize = true;
                        }
                        ax.needs_flush = true;
                        ax.chat_ui_dirty = true;
                        if interaction::should_materialize_chat_ui(
                            &AgentEvent::ContentDelta { text: String::new() },
                            streaming_before,
                            kind_switch,
                        ) {
                            force_materialize = true;
                        }
                    }
                    AgentEvent::ReasoningDelta { text } => {
                        let kind_switch =
                            interaction::apply_reasoning_content_delta(&mut ax.session, &text);
                        ax.needs_flush = true;
                        ax.chat_ui_dirty = true;
                        if interaction::should_materialize_chat_ui(
                            &AgentEvent::ReasoningDelta { text: String::new() },
                            streaming_before,
                            kind_switch,
                        ) {
                            force_materialize = true;
                        }
                    }
                    AgentEvent::ToolUse { id, name, input } => {
                        // Attribute a change folder to this session at the
                        // causal moment: an exploration whose agent runs
                        // `ds create change` owns the folder it creates.
                        // Match on the command string, not the tool name —
                        // Claude uses "Bash", grok uses "run_terminal_command".
                        if ax.scope_kind == scope::ScopeKind::Exploration
                            && let Some(slug) = parse_create_change(&input)
                        {
                            staged_binding = Some((slug, ax.session.scope.clone()));
                        }
                        interaction::flush_all_pending(&mut ax.session);
                        interaction::reset_answer_thrash(&mut ax.session);
                        ax.session.messages.push(chat_store::ChatMessage {
                            role: chat_store::Role::Assistant,
                            content: vec![chat_store::ContentBlock::ToolUse { id, name, input }],
                            timestamp: String::new(),
                            is_priming: false,
                        });
                        ax.needs_flush = true;
                        ax.chat_ui_dirty = true;
                        force_materialize = true;
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
                            is_priming: false,
                        });
                        ax.needs_flush = true;
                        ax.chat_ui_dirty = true;
                        force_materialize = true;
                    }
                    AgentEvent::UserChoiceRequest {
                        correlation_id,
                        prompt,
                        options,
                        allow_cancel,
                    } => {
                        interaction::apply_user_choice_request(
                            ax,
                            correlation_id,
                            prompt,
                            options,
                            allow_cancel,
                        );
                        // Chips appear while the turn stays open.
                        force_materialize = true;
                    }
                    AgentEvent::TurnComplete => {
                        interaction::flush_all_pending(&mut ax.session);
                        interaction::reset_answer_thrash(&mut ax.session);
                        ax.session.is_streaming = false;
                        // Drop any leftover chips if the turn ended without answer.
                        interaction::clear_user_choice_shell(ax);
                        ax.chat_ui_dirty = true;
                        force_materialize = true;
                        // Rebuild next actions from the new trailing assistant text.
                        // Turn boundary: ghost starts at first ranked action.
                        ax.refresh_next_actions(true);
                        // Detect the AGENTS.md priming turn and stage the
                        // user's actual first message for dispatch in the
                        // post-match block (where we can borrow `highlighter`
                        // alongside the session). Skip the title summariser
                        // on this turn — the priming exchange isn't the
                        // user's intent and would yield a useless title.
                        let was_priming = ax.priming_in_flight;
                        if was_priming {
                            ax.priming_in_flight = false;
                        }
                        if let Err(e) = chat_store::save_session(&ax.session, proj_root.as_deref())
                        {
                            tracing::error!("failed to save chat session: {e}");
                        }
                        // Turn-boundary flush is authoritative — the debounced
                        // eager flag is now satisfied.
                        ax.needs_flush = false;
                        // Kick off a one-shot title summary after the first
                        // turn whose user message isn't a bare slash command.
                        // Bare-command-only sessions defer summarisation
                        // until a real message arrives — `title.is_none()`
                        // ensures this block runs again on each TurnComplete
                        // until that happens. Only for change / exploration
                        // scopes; caps and codex don't get summarised.
                        if !was_priming
                            && ax.session.title.is_none()
                            && matches!(
                                ax.scope_kind,
                                scope::ScopeKind::Change | scope::ScopeKind::Exploration
                            )
                            && let Some(handle) = ax.agent_handle.clone()
                            && let Some(target) =
                                chat_store::title_summarization_target(&ax.session)
                        {
                            title_task_input = Some((
                                handle,
                                ax.session.scope.clone(),
                                ax.scope_kind,
                                target.message,
                                target.command_hint_source,
                                ax.idea_description.clone(),
                            ));
                        }
                        // Reply-suggestion oneshot: gated by agent input hints
                        // and empty next-action list (skip model when ghost wins).
                        if let Some((assistant, user)) =
                            default_prompts::last_assistant_and_user(&ax.session)
                        {
                            let has_assistant = !assistant.trim().is_empty();
                            if default_prompts::should_begin_reply_oneshot(
                                agent_input_hints,
                                was_priming,
                                has_assistant,
                                ax.next_actions.is_empty(),
                            ) && let Some(handle) = ax.agent_handle.clone()
                            {
                                ax.begin_default_prompts_oneshot();
                                let prompts_gen = ax.default_prompts_gen;
                                reply_task_input = Some((
                                    handle,
                                    assistant,
                                    user,
                                    prompts_gen,
                                ));
                            }
                        }
                        // Shell empty until oneshot settles (or clear if ineligible).
                        interaction::sync_oneshot_chips(ax, agent_input_hints);
                    }
                    AgentEvent::Error(msg) => {
                        // Defensive: stringified session-not-found (older
                        // workers / odd protocol shapes) still recovers.
                        if duckchat::Error::Protocol(msg.clone()).is_session_not_found() {
                            tracing::warn!(
                                key,
                                "agent session not found (via Error); recovering as fresh session"
                            );
                            recover_lost_session = true;
                        } else {
                            tracing::error!(key, "agent error: {msg}");
                            ax.session.is_streaming = false;
                            interaction::reset_answer_thrash(&mut ax.session);
                            interaction::clear_user_choice_shell(ax);
                            // Drop priming state so a failed AGENTS.md priming
                            // doesn't fire its follow-up against a half-broken
                            // session. The user will retype if they want to retry.
                            ax.priming_in_flight = false;
                            ax.pending_followup_prompt = None;
                            ax.session.messages.push(chat_store::ChatMessage {
                                role: chat_store::Role::System,
                                content: vec![chat_store::ContentBlock::Text(format!(
                                    "Error: {msg}"
                                ))],
                                timestamp: String::new(),
                                is_priming: false,
                            });
                            ax.chat_ui_dirty = true;
                            force_materialize = true;
                        }
                    }
                    AgentEvent::SessionNotFound => {
                        // grok session/load (or equivalent) could not find the
                        // stored id — typically a cwd-key mismatch or prune.
                        // Drop the dead id and re-dispatch the last user turn
                        // with a history preamble so the chat unblocks.
                        tracing::warn!(
                            key,
                            "agent session not found; recovering as fresh session"
                        );
                        recover_lost_session = true;
                    }
                    AgentEvent::SessionIdUpdated { session_id } => {
                        // Stamp the id with the harness that produced it so a
                        // later harness switch knows the id is foreign and starts
                        // a fresh agent session instead of a doomed `session/load`.
                        let harness = ax.effective_harness();
                        ax.session.session_harness = Some(harness);
                        ax.session.agent_session_id = Some(session_id);
                    }
                    AgentEvent::UsageUpdate {
                        input_tokens,
                        output_tokens,
                    } => {
                        if input_tokens > 0 {
                            ax.agent_input_tokens = input_tokens;
                        }
                        if output_tokens > 0 {
                            ax.agent_output_tokens = output_tokens;
                        }
                        // Write-through so the next turn-boundary / eager save
                        // persists last-known fill. Do not set needs_flush for
                        // usage alone (avoid rewriting the session on every
                        // telemetry tick when messages are unchanged).
                        ax.session.context_tokens =
                            ax.agent_input_tokens + ax.agent_output_tokens;
                    }
                    AgentEvent::ProcessExited => {
                        tracing::info!(key, "agent process exited");
                        ax.agent_handle = None;
                        ax.session.is_streaming = false;
                        interaction::reset_answer_thrash(&mut ax.session);
                        interaction::clear_user_choice_shell(ax);
                        // Drop any priming state — without a handle the
                        // follow-up can't dispatch, and stale flags would
                        // confuse the next reconnect.
                        ax.priming_in_flight = false;
                        ax.pending_followup_prompt = None;
                        // Drop in-flight oneshot list/chips when the worker is
                        // gone with no DefaultPromptsReady settle.
                        ax.clear_agent_default_prompts();
                        // Paint any deferred stream tail and drop streaming chrome.
                        force_materialize = true;
                    }
                }
            }
            if let Some((slug, exploration_id)) = staged_binding {
                state.change.pending_bindings.insert(slug, exploration_id);
            }
            let State {
                interactions,
                highlighter,
                ..
            } = state;
            let ax = resolve_session_mut(interactions, &key);
            let mut should_snap_to_bottom = false;
            if let Some(ax) = ax {
                if recover_lost_session {
                    interaction::recover_from_lost_session(ax, highlighter);
                    force_materialize = true;
                }
                let is_streaming = ax.session.is_streaming;
                // Materialize immediately on structural events / kind switches /
                // recovery; pure content while streaming waits for StreamTick.
                if force_materialize || (ax.chat_ui_dirty && !is_streaming) {
                    interaction::materialize_chat_ui(ax, highlighter);
                    should_snap_to_bottom = ax.stick_to_bottom;
                }
                if !is_streaming {
                    ax.esc_count = 0;
                    // An abrupt turn end (Error / ProcessExited) leaves
                    // streamed messages dirty without a turn-boundary save —
                    // persist them now so the turn's tail survives.
                    if ax.needs_flush
                        && interaction::persist_session_snapshot(&ax.session, proj_root.as_deref())
                    {
                        ax.needs_flush = false;
                    }
                    // Order matters: dispatch the AGENTS.md priming follow-up
                    // before any queued message so the user's intended first
                    // turn lands ahead of anything they typed while priming
                    // was streaming. `send_prompt_text` flips `is_streaming`
                    // back on, so the queue branch below correctly defers.
                    if ax.agent_handle.is_some()
                        && let Some(text) = ax.pending_followup_prompt.take()
                    {
                        interaction::send_prompt_text(ax, text, highlighter);
                    } else if ax.agent_handle.is_some()
                        && let Some(q) = ax.queue_editor.take()
                    {
                        // Auto-flush a queued message once the current turn is
                        // done (natural completion or user-triggered interrupt).
                        // Only flush if the agent is still attached — on
                        // ProcessExited the handle is gone and we'd lose the text.
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

            let mut follow_tasks: Vec<Task<Message>> = vec![snap_task];

            if let Some((
                handle,
                scope_key,
                scope_kind,
                user,
                command_hint_source,
                idea_description,
            )) = title_task_input
            {
                use duckchat::ContextHook;
                let mut hints = Vec::new();
                // Slash-command hint comes from the most recent command-bearing
                // turn (which may be `user` itself, an earlier bare-command
                // turn, or absent). `build_hint` re-extracts the command name
                // from the source message text.
                if let Some(src) = command_hint_source.as_deref()
                    && let Some(hint) = title_hints::build_hint(src, &scope_key, &state.project)
                {
                    hints.push(hint);
                }
                let scope_input = scope::SessionScope {
                    kind: scope_kind,
                    scope_key: scope_key.clone(),
                    // Title generation only needs the scope name, not full
                    // lifecycle facts — keep the hint terse.
                    change_facts: None,
                };
                if let Some(out) = scope::CurrentScopeHook.compute(&scope_input) {
                    hints.push(out.text);
                }
                if let Some(idea_hint) = title_hints::build_idea_hint(idea_description.as_deref()) {
                    hints.push(idea_hint);
                }
                let mut req = duckchat::TitleRequest::new(user);
                req.context_hints = hints;
                let route_key = key.clone();
                // `AgentHandle` is `Clone`; move the clone into the async task
                // so title summary uses the chat's oneshot runtime.
                let work = async move {
                    handle
                        .title_summary(req)
                        .await
                        .map_err(|e| e.to_string())
                };
                follow_tasks.push(Task::perform(work, move |result| {
                    Message::SessionTitleReady {
                        key: route_key.clone(),
                        result,
                    }
                }));
            }

            if let Some((handle, assistant, user, prompts_gen)) = reply_task_input {
                let route_key = key.clone();
                let work = async move {
                    let mut req = duckchat::ReplySuggestionRequest::new(assistant);
                    req.user_message = user;
                    handle
                        .reply_suggestions(req)
                        .await
                        .map_err(|e| e.to_string())
                };
                follow_tasks.push(Task::perform(work, move |result| {
                    Message::DefaultPromptsReady {
                        key: route_key.clone(),
                        prompts_gen,
                        result,
                    }
                }));
            }

            return Task::batch(follow_tasks);
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
        Message::DefaultPromptsReady {
            key,
            prompts_gen,
            result,
        } => {
            let agent_input_hints = state.config.chat.agent_input_hints;
            let Some(ax) = state.agent_session_mut(&key) else {
                return Task::none();
            };
            // Map Err to a string so the pure helper can settle either arm.
            // Ok or Err (including oneshot timeout) both settle when gen matches.
            let result = result.map_err(|e| {
                tracing::warn!(key, "reply suggestions failed: {e}");
                e
            });
            let Some(list) = default_prompts::apply_oneshot_if_current(
                ax.default_prompts_gen,
                prompts_gen,
                result,
            ) else {
                // Superseded generation — leave list and readiness unchanged.
                return Task::none();
            };
            // Oneshot list (parse only; empty on fail) is ready. Next actions
            // are independent and not updated here. Sync chips when eligible.
            ax.agent_default_prompts = list;
            ax.default_prompts_pending = false;
            interaction::sync_oneshot_chips(ax, agent_input_hints);
        }
        Message::ThemeChanged(mode) => {
            theme::set_mode(mode);
            return rehighlight_all(state);
        }
        Message::StreamTick => {
            widget::streaming_indicator::bump_tick();
            // Drain deferred pure-content materialize only while the user is
            // following the live answer (stick-to-bottom). Scrolled-up readers
            // keep session text accumulating without 10 Hz column rebuilds.
            let mut should_snap = false;
            {
                let State {
                    interactions,
                    highlighter,
                    ..
                } = state;
                for ix in interactions.values_mut() {
                    let active = ix.active_session;
                    for (i, ax) in ix.sessions.iter_mut().enumerate() {
                        if interaction::should_materialize_on_stream_tick(
                            ax.session.is_streaming,
                            ax.chat_ui_dirty,
                            ax.stick_to_bottom,
                        ) {
                            interaction::materialize_chat_ui(ax, highlighter);
                            if i == active {
                                should_snap = true;
                            }
                        }
                    }
                }
            }
            if should_snap {
                return iced::widget::operation::snap_to_end(
                    widget::agent_chat::CHAT_SCROLLABLE_ID,
                );
            }
        }
        Message::TerminalAutoscrollTick => {
            for ix in state.interactions.values_mut() {
                for tt in &mut ix.terminals {
                    if tt.state.is_drag_autoscrolling() {
                        tt.state.drag_autoscroll_step();
                    }
                }
            }
        }
        Message::FlushTick => {
            let proj_root = state.project.project_root.clone();
            for ix in state.interactions.values_mut() {
                interaction::flush_dirty_sessions(ix, proj_root.as_deref());
            }
        }
        Message::WindowCloseRequested(id) => {
            // Force a final flush of every session before the window closes so
            // a clean quit never drops an in-flight turn, then let the window
            // actually close (we suppressed the default via
            // `exit_on_close_request(false)`).
            let proj_root = state.project.project_root.clone();
            for ix in state.interactions.values() {
                interaction::flush_sessions(ix, proj_root.as_deref());
            }
            return iced::window::close(id);
        }
        Message::WindowResized(size) => {
            state.window_width = size.width;
            for ix in state.interactions.values_mut() {
                interaction::rebalance_uncustomized(ix, size.width);
            }
        }
        Message::KeyPress(key, mods, text) => {
            // Disarm any pending dirty-tab close on the next keypress.
            // Snapshot first so Cmd-W can still consult the previously
            // armed value within this same key handler. Any other key
            // simply lets the arm decay.
            let armed_tab_snapshot = state.armed_tab_close.take();
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

            // Cmd+N — `keybinds::keybind_new` decides what "new" means given
            // current focus + area. Content-column focus opens the new-file
            // modal; chat / no focus falls through to area-specific behavior
            // (add idea, new chat session, add exploration). Cmd+Shift+N
            // (spawn new window) was intercepted above.
            if mods.command()
                && !mods.shift()
                && key == keyboard::Key::Character("n".into())
                && let Some(action) = keybinds::keybind_new(state)
            {
                use keybinds::NewAction;
                return match action {
                    NewAction::OpenNewFile => {
                        let seed = new_file_seed_path(state);
                        update(state, Message::NewFile(widget::new_file::Msg::OpenAt(seed)))
                    }
                    NewAction::AddIdea => {
                        update(state, Message::Ideas(area::ideas::Message::AddIdea))
                    }
                    NewAction::NewChatSession(rk) => {
                        dispatch_interaction_msg(state, &rk, interaction::Msg::NewSession)
                    }
                    NewAction::AddExploration => update(
                        state,
                        Message::Change(area::change::Message::AddExploration),
                    ),
                };
            }

            // Cmd+S — `keybinds::keybind_save` only flags focus-conditional
            // saves. The generic in-editor save path runs through the editor
            // widget's own `SaveRequested` action and never reaches here.
            if mods.command()
                && matches!(&key, keyboard::Key::Character(c) if c.eq_ignore_ascii_case("s"))
                && let Some(action) = keybinds::keybind_save(state)
            {
                use keybinds::SaveAction;
                return match action {
                    SaveAction::SaveIdeaBody => {
                        update(state, Message::Ideas(area::ideas::Message::SaveBody))
                    }
                };
            }

            // Cmd+Shift+F: open project-wide text search.
            if mods.command()
                && mods.shift()
                && matches!(&key, keyboard::Key::Character(c) if c.eq_ignore_ascii_case("f"))
            {
                return update(state, Message::TextSearch(widget::text_search::Msg::Open));
            }

            // Cmd+F: open local find modal targeting the focused column.
            // No-op if neither editor nor chat owns focus.
            if mods.command()
                && !mods.shift()
                && matches!(&key, keyboard::Key::Character(c) if c.eq_ignore_ascii_case("f"))
            {
                return update(state, Message::FindOpen);
            }

            // When the find modal is open, route Esc / Enter / preview nav.
            // Char input flows to the embedded text_input naturally.
            if state.find_modal.visible {
                use keyboard::key::Named;
                match &key {
                    keyboard::Key::Named(Named::Escape) => {
                        return update(state, Message::Find(widget::find::Msg::Cancel));
                    }
                    keyboard::Key::Named(Named::Enter) => {
                        return update(state, Message::Find(widget::find::Msg::Commit));
                    }
                    keyboard::Key::Named(Named::ArrowDown) => {
                        let _ = update(state, Message::Find(widget::find::Msg::PreviewSelectNext));
                    }
                    keyboard::Key::Named(Named::ArrowUp) => {
                        let _ = update(state, Message::Find(widget::find::Msg::PreviewSelectPrev));
                    }
                    _ if mods.control() && key == keyboard::Key::Character("n".into()) => {
                        let _ = update(state, Message::Find(widget::find::Msg::PreviewSelectNext));
                    }
                    _ if mods.control() && key == keyboard::Key::Character("p".into()) => {
                        let _ = update(state, Message::Find(widget::find::Msg::PreviewSelectPrev));
                    }
                    _ => {}
                }
                return Task::none();
            }

            // Ctrl-N / Ctrl-P navigation when a find is active for the
            // focused column. Lower priority than the chat completion popup
            // (handled later via `handle_agent_chat_key`), so a visible
            // completion still wins.
            if mods.control()
                && !mods.shift()
                && (key == keyboard::Key::Character("n".into())
                    || key == keyboard::Key::Character("p".into()))
                && let Some(target) = keybinds::keybind_find(state)
                && state.find_states.contains_key(&target)
            {
                let completion_eats = state
                    .active_scope()
                    .and_then(|s| state.interactions.get(&s))
                    .and_then(|ix| ix.active())
                    .is_some_and(|ax| ax.chat_completion.visible);
                if !completion_eats {
                    let dir = if key == keyboard::Key::Character("n".into()) {
                        widget::find::NavDir::Next
                    } else {
                        widget::find::NavDir::Prev
                    };
                    return update(
                        state,
                        Message::Find(widget::find::Msg::Navigate(target, dir)),
                    );
                }
            }

            // When text search is visible, route navigation keys.
            if state.text_search.visible {
                use keyboard::key::Named;
                match &key {
                    keyboard::Key::Named(Named::Escape) => {
                        let _ = update(state, Message::TextSearch(widget::text_search::Msg::Close));
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

            // When new-file modal is visible, route navigation keys. Tab
            // completes the highlighted candidate; Enter resolves to either
            // open (existing file) or create (new file), then opens it as a
            // tab. Ctrl-N/P navigate the candidate list.
            if state.new_file.visible {
                use keyboard::key::Named;
                match &key {
                    keyboard::Key::Named(Named::Escape) => {
                        let _ = update(state, Message::NewFile(widget::new_file::Msg::Close));
                    }
                    keyboard::Key::Named(Named::Tab) => {
                        return update(state, Message::NewFile(widget::new_file::Msg::TabComplete));
                    }
                    keyboard::Key::Named(Named::Enter) => {
                        return update(state, Message::NewFile(widget::new_file::Msg::Confirm));
                    }
                    keyboard::Key::Named(Named::ArrowDown) => {
                        let _ = update(state, Message::NewFile(widget::new_file::Msg::SelectNext));
                    }
                    keyboard::Key::Named(Named::ArrowUp) => {
                        let _ = update(state, Message::NewFile(widget::new_file::Msg::SelectPrev));
                    }
                    _ if mods.control() && key == keyboard::Key::Character("n".into()) => {
                        let _ = update(state, Message::NewFile(widget::new_file::Msg::SelectNext));
                    }
                    _ if mods.control() && key == keyboard::Key::Character("p".into()) => {
                        let _ = update(state, Message::NewFile(widget::new_file::Msg::SelectPrev));
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
                return update(state, Message::Ideas(area::ideas::Message::CancelTagInput));
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

            // ⌘↑/↓/←/→ chat landmarks — after modal handlers so open modals
            // keep arrow ownership. Bare arrows still go to the composer.
            if mods.command()
                && !mods.shift()
                && !mods.alt()
                && keybinds::keybind_chat_landmarks(state)
            {
                use keyboard::key::Named;
                use keybinds::ChatLandmarkAction;
                let action = match &key {
                    keyboard::Key::Named(Named::ArrowUp) => Some(ChatLandmarkAction::HistoryTop),
                    keyboard::Key::Named(Named::ArrowDown) => {
                        Some(ChatLandmarkAction::HistoryBottom)
                    }
                    keyboard::Key::Named(Named::ArrowLeft) => Some(ChatLandmarkAction::PrevAnswer),
                    keyboard::Key::Named(Named::ArrowRight) => Some(ChatLandmarkAction::NextAnswer),
                    _ => None,
                };
                if let Some(action) = action {
                    return apply_chat_landmark(state, action);
                }
            }

            // Cmd-K — pin the active session's tentative selection.
            // `keybinds::keybind_pin_selection` decides whether the focus
            // is right; the live runtime check (was there actually a
            // tentative to pin?) stays here because it needs `&mut` and is
            // outcome-driven, not focus-driven.
            if mods.command()
                && !mods.shift()
                && matches!(&key, keyboard::Key::Character(c) if c.eq_ignore_ascii_case("k"))
                && keybinds::keybind_pin_selection(state)
                && let Some(scope) = state.active_scope()
                && let Some(ix) = state.interactions.get_mut(&scope)
                && let Some(ax) = ix.active_mut()
                && interaction::pin_tentative(ax)
            {
                // Pinning leaves the source editor's anchor untouched so
                // the visual selection lingers; clear it on the active
                // content tab too so the user gets a clean "next selection
                // is a fresh tentative" gesture.
                if let Some(tab) = state.tabs.active_tab_mut() {
                    match &mut tab.view {
                        tab_bar::TabView::Editor { editor, .. }
                        | tab_bar::TabView::Diff { editor, .. } => {
                            editor.anchor = None;
                        }
                        tab_bar::TabView::SearchStack { .. } => {}
                    }
                }
                return Task::none();
            }

            // Cmd-R — clear all attachments on the active session.
            if mods.command()
                && !mods.shift()
                && matches!(&key, keyboard::Key::Character(c) if c.eq_ignore_ascii_case("r"))
                && keybinds::keybind_clear_attachments(state)
                && let Some(scope) = state.active_scope()
                && let Some(ix) = state.interactions.get_mut(&scope)
                && let Some(ax) = ix.active_mut()
            {
                interaction::clear_all_attachments(ax);
                return Task::none();
            }

            // Cmd-W — close the active file tab. `keybinds::keybind_close`
            // gates on focus (chat input / terminal don't claim cmd-w) and
            // returns the logical tab index to close. Mirrors the X-click
            // flow: dirty unarmed → arm; dirty armed → close; clean → close.
            if mods.command()
                && !mods.shift()
                && matches!(&key, keyboard::Key::Character(c) if c.eq_ignore_ascii_case("w"))
                && let Some(idx) = keybinds::keybind_close(state)
            {
                let logical_to_active = |i: usize| -> tab_bar::ActiveTab {
                    if state.tabs.preview.is_some() {
                        if i == 0 {
                            tab_bar::ActiveTab::Preview
                        } else {
                            tab_bar::ActiveTab::File(i - 1)
                        }
                    } else {
                        tab_bar::ActiveTab::File(i)
                    }
                };
                let is_dirty = match logical_to_active(idx) {
                    tab_bar::ActiveTab::Preview => false,
                    tab_bar::ActiveTab::File(fi) => state
                        .tabs
                        .file_tabs
                        .get(fi)
                        .map(|t| matches!(&t.view, tab_bar::TabView::Editor { editor, .. } if editor.dirty))
                        .unwrap_or(false),
                };
                if is_dirty && armed_tab_snapshot != Some(idx) {
                    return update(state, Message::TabArmClose(idx));
                }
                return update(state, Message::TabClose(idx));
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
                let agent_input_hints = state.config.chat.agent_input_hints;
                if agent_chat_active && let Some(ix) = state.interaction_mut(routing_key) {
                    match interaction::handle_agent_chat_key(
                        ix,
                        &key,
                        mods,
                        agent_input_hints,
                    ) {
                        interaction::AgentChatKeyResult::Handled => return Task::none(),
                        interaction::AgentChatKeyResult::Dispatch(msg) => {
                            // Tab-cycling defaults should leave the caret in the
                            // chat input so Enter still works without a re-click.
                            let refocus = matches!(
                                &msg,
                                widget::agent_chat::Msg::CycleNextAction(_)
                            );
                            let dispatch = dispatch_interaction_msg(
                                state,
                                routing_key,
                                interaction::Msg::AgentChat(msg),
                            );
                            if refocus {
                                return Task::batch([dispatch, focus_chat_input()]);
                            }
                            return dispatch;
                        }
                        interaction::AgentChatKeyResult::NotHandled => {}
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

/// Mirror the active content tab's selection into the active chat session's
/// tentative attachment. Called after every `TabContent` editor action so
/// the chip above the chat input tracks the live selection.
///
/// Skipped (no-op) when there's no chat session, no content tab, or the
/// active tab is a search stack (multiple editors, no single source).
fn sync_tentative_from_active_tab(state: &mut State) {
    let Some(scope) = state.active_scope() else {
        return;
    };
    let Some(tab) = state.tabs.active_tab() else {
        return;
    };
    let display_path = tab_display_path(tab, state.project.project_root.as_deref());
    let editor = match &tab.view {
        tab_bar::TabView::Editor { editor, .. } | tab_bar::TabView::Diff { editor, .. } => editor,
        tab_bar::TabView::SearchStack { .. } => return,
    };
    // Snapshot the editor view once to avoid simultaneous &mut state.tabs and
    // &mut state.interactions borrows.
    let editor_clone = editor.clone();
    if let Some(ix) = state.interactions.get_mut(&scope)
        && let Some(ax) = ix.active_mut()
    {
        ax.chat_input_focused = false;
        interaction::set_tentative_from_tab(ax, &editor_clone, display_path);
    }
}

/// User-facing label for a content tab — used by selection-attachment chips
/// and the agent's context payload. Resolves to a project-relative path
/// when possible, falling back to the absolute path or the tab title for
/// non-file tabs (ideas, etc.).
fn tab_display_path(tab: &tab_bar::Tab, project_root: Option<&Path>) -> String {
    let path = match &tab.view {
        tab_bar::TabView::Editor { path, .. } => path.as_deref(),
        tab_bar::TabView::Diff { path, .. } => Some(path.as_path()),
        tab_bar::TabView::SearchStack { .. } => None,
    };
    if let Some(p) = path {
        if let Some(root) = project_root
            && let Ok(rel) = p.strip_prefix(root)
        {
            return rel.display().to_string();
        }
        return p.display().to_string();
    }
    if tab.id.starts_with("idea:") {
        return format!("idea: {}", tab.title);
    }
    tab.title.clone()
}

/// Restore the chat scrollable's viewport for the area we just switched to.
/// `AgentSession` survives area switches but the iced `Scrollable` widget
/// is rebuilt fresh on each view, defaulting back to (0, 0). We replay the
/// last seen `absolute_offset.y` (captured by the `ChatScrolled` handler),
/// or snap to the end when the user was sticking to the bottom.
fn restore_chat_scroll(state: &State) -> Task<Message> {
    let Some(scope) = state.active_scope() else {
        return Task::none();
    };
    let Some(ix) = state.interactions.get(&scope) else {
        return Task::none();
    };
    let Some(ax) = ix.active() else {
        return Task::none();
    };
    if ax.stick_to_bottom {
        iced::widget::operation::snap_to_end(widget::agent_chat::CHAT_SCROLLABLE_ID)
    } else {
        let y = ax.last_chat_offset_y.unwrap_or(0.0);
        iced::widget::operation::scroll_to(
            widget::agent_chat::CHAT_SCROLLABLE_ID,
            iced::widget::scrollable::AbsoluteOffset { x: 0.0, y },
        )
    }
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

/// True when any chat session has an accumulated edge auto-scroll delta from a
/// drag that ran past the chat fold, awaiting drain into a `scroll_to`.
fn has_pending_chat_autoscroll(state: &State) -> bool {
    state
        .interactions
        .values()
        .any(|ix| ix.sessions.iter().any(|ax| ax.pending_chat_autoscroll.is_some()))
}

/// Drain each session's pending chat auto-scroll delta into an absolute scroll
/// on the chat scrollable. The delta advances `last_chat_offset_y` so the
/// scroll-preservation replay stays consistent with the new position.
fn take_pending_chat_autoscroll(state: &mut State) -> Task<Message> {
    let mut task = Task::none();
    for ix in state.interactions.values_mut() {
        for ax in &mut ix.sessions {
            if let Some(dy) = ax.pending_chat_autoscroll.take() {
                let y = (ax.last_chat_offset_y.unwrap_or(0.0) + dy).max(0.0);
                ax.last_chat_offset_y = Some(y);
                task = Task::batch([
                    task,
                    iced::widget::operation::scroll_to(
                        widget::agent_chat::CHAT_SCROLLABLE_ID,
                        iced::widget::scrollable::AbsoluteOffset { x: 0.0, y },
                    ),
                ]);
            }
        }
    }
    task
}

/// Snapshot of the active chat session's scroll intent — captured before
/// `update` runs and replayed afterwards to neutralize layout-driven
/// resets. Iced 0.14's `Scrollable` re-clamps offset on bounds/content
/// changes, and the `on_scroll` viewport notifications fire for both
/// user scrolls and content reflows. We can't reliably tell them apart
/// inside the handler, so instead we treat *all* messages other than
/// `ChatScrolled` itself as potential layout-changers and re-issue the
/// last user-intended position. For `ChatScrolled` we skip — the user's
/// own scroll *is* the new intent.
#[derive(Clone, Copy)]
enum ChatScrollSnapshot {
    StickToBottom,
    At(f32),
}

fn capture_chat_scroll_snapshot(state: &State) -> Option<ChatScrollSnapshot> {
    let scope = state.active_scope()?;
    let ix = state.interactions.get(&scope)?;
    let ax = ix.active()?;
    if ax.stick_to_bottom {
        Some(ChatScrollSnapshot::StickToBottom)
    } else {
        ax.last_chat_offset_y.map(ChatScrollSnapshot::At)
    }
}

fn replay_chat_scroll(snap: ChatScrollSnapshot) -> Task<Message> {
    match snap {
        ChatScrollSnapshot::StickToBottom => {
            iced::widget::operation::snap_to_end(widget::agent_chat::CHAT_SCROLLABLE_ID)
        }
        ChatScrollSnapshot::At(y) => iced::widget::operation::scroll_to(
            widget::agent_chat::CHAT_SCROLLABLE_ID,
            iced::widget::scrollable::AbsoluteOffset { x: 0.0, y },
        ),
    }
}

/// True when the message is the chat scrollable's own viewport notification,
/// or when the message intentionally drives chat scroll itself (e.g. local
/// find navigation). Those messages own the scroll intent — the wrapper
/// must not override them with the pre-message snapshot.
fn is_chat_scroll_message(msg: &Message) -> bool {
    fn is_chat_scrolled(im: &interaction::Msg) -> bool {
        matches!(
            im,
            interaction::Msg::AgentChat(widget::agent_chat::Msg::ChatScrolled(_))
        )
    }
    match msg {
        Message::Interaction(im) => is_chat_scrolled(im),
        Message::Change(area::change::Message::Interaction(im)) => is_chat_scrolled(im),
        Message::Caps(area::caps::Message::Interaction(im)) => is_chat_scrolled(im),
        Message::Codex(area::codex::Message::Interaction(im)) => is_chat_scrolled(im),
        Message::Ideas(area::ideas::Message::Interaction(im)) => is_chat_scrolled(im),
        // Local-find commit/navigate is a deliberate scroll into a chat
        // match — letting the snapshot wrapper restore the prior offset
        // would visually undo the find jump.
        Message::Find(widget::find::Msg::Commit)
        | Message::Find(widget::find::Msg::Navigate(_, _)) => true,
        _ => false,
    }
}

/// Outer entry point for `iced::application`. Wraps `update` so every
/// non-`ChatScrolled` message captures the chat's scroll intent before
/// dispatch and replays it after — preventing layout-driven resets
/// (modal open/close, content column appearing/disappearing, etc.) from
/// silently jumping the chat to the top.
/// Whether a message opens an artifact / file into the content column from a
/// list click. Used to re-expand a collapsed content column even when the
/// click re-selects the file already shown (so no active-tab change occurs).
fn message_opens_content(message: &Message) -> bool {
    match message {
        Message::Change(m) => matches!(
            m,
            area::change::Message::SelectItem(_)
                | area::change::Message::SelectChangedFile(_)
                | area::change::Message::SelectExplorerFile(_)
        ),
        Message::Caps(m) => matches!(m, area::caps::Message::SelectItem(_)),
        Message::Codex(m) => matches!(m, area::codex::Message::SelectItem(_)),
        Message::Ideas(m) => matches!(m, area::ideas::Message::SelectIdea(_)),
        _ => false,
    }
}

fn update_with_scroll_preservation(state: &mut State, message: Message) -> Task<Message> {
    // Chrome pad measure messages must not snapshot/replay scroll — they only
    // adjust an in-scroll spacer.
    let is_chrome_layout = is_chrome_layout_message(&message);
    let snapshot = if is_chat_scroll_message(&message) || is_chrome_layout {
        None
    } else {
        capture_chat_scroll_snapshot(state)
    };
    state.chat_scroll_overridden = false;
    let tab_before = state.tabs.active_tab().map(|t| t.id.clone());
    // A list click that opens content re-expands the content column even when
    // it re-selects the already-active file (no tab change to observe).
    let opens_content = message_opens_content(&message);
    let task = update(state, message);
    // Whenever this tick changed the active tab — regardless of which route
    // did it (tab bar, file finder, search, cmd-click, tab close) — reveal
    // the newly active file in the Files explorer.
    let tab_after = state.tabs.active_tab().map(|t| t.id.as_str());
    let tab_changed = tab_before.as_deref() != tab_after;
    // Opening or switching to a content tab re-expands the content column if
    // the door had been dragged fully open over it — clicking a file in the
    // list brings the content back.
    if (opens_content || (tab_changed && tab_after.is_some()))
        && let Some(scope) = state.active_scope()
        && let Some(ix) = state.interactions.get_mut(&scope)
    {
        ix.content_collapsed = false;
    }
    let task = if tab_changed {
        Task::batch([task, reveal_active_file_in_explorer(state)])
    } else {
        task
    };
    // If a Find action drove a chat scroll during this tick (e.g. ctrl-n/p
    // routed via KeyPress → Find(Navigate)), skip the replay — its job is
    // to *preserve* the user's prior intent, not to undo a deliberate
    // scroll-to-match.
    if state.chat_scroll_overridden {
        state.chat_scroll_overridden = false;
        return Task::batch([task, maybe_measure_chrome_pad(state)]);
    }
    // A chat-fold drag accumulated a deliberate scroll this tick. Issue it and
    // skip the snapshot replay — replaying the pre-update offset would undo the
    // scroll every frame the drag holds past the edge.
    if has_pending_chat_autoscroll(state) {
        return Task::batch([
            task,
            take_pending_chat_autoscroll(state),
            maybe_measure_chrome_pad(state),
        ]);
    }
    let task = match snapshot {
        Some(snap) => Task::batch([task, replay_chat_scroll(snap)]),
        None => task,
    };
    // After layout-affecting updates, measure scroll bounds so the bottom-pin
    // pad works when content still fits the viewport (no on_scroll from iced).
    // Skip re-measure on ChromeLayout itself to avoid a tight loop; one more
    // measure is scheduled only when pad actually changes (handled there).
    if is_chrome_layout {
        task
    } else {
        Task::batch([task, maybe_measure_chrome_pad(state)])
    }
}

fn is_chrome_layout_message(msg: &Message) -> bool {
    fn is_layout(im: &interaction::Msg) -> bool {
        matches!(
            im,
            interaction::Msg::AgentChat(widget::agent_chat::Msg::ChromeLayout { .. })
        )
    }
    match msg {
        Message::Interaction(im) => is_layout(im),
        Message::Change(area::change::Message::Interaction(im)) => is_layout(im),
        Message::Caps(area::caps::Message::Interaction(im)) => is_layout(im),
        Message::Codex(area::codex::Message::Interaction(im)) => is_layout(im),
        Message::Ideas(area::ideas::Message::Interaction(im)) => is_layout(im),
        _ => false,
    }
}

/// When fast-response chips are visible on the active chat, schedule a bounds
/// measure so the in-scroll bottom-pin pad can update. No-op otherwise.
fn maybe_measure_chrome_pad(state: &State) -> Task<Message> {
        let Some(scope) = state.active_scope() else {
        return Task::none();
    };
    let Some(ix) = state.interactions.get(&scope) else {
        return Task::none();
    };
    let Some(ax) = ix.active() else {
        return Task::none();
    };
    let input_empty = ax.chat_input.text().trim().is_empty();
    if !crate::fast_response::visible(
        ax.session.is_streaming,
        ax.is_awaiting_user,
        input_empty,
        &ax.fast_response,
    ) {
        return Task::none();
    }
    let area = state.active_area;
    widget::agent_chat::measure_scroll_bounds().map(move |(viewport_h, content_h)| {
        let im = interaction::Msg::AgentChat(widget::agent_chat::Msg::ChromeLayout {
            viewport_h,
            content_h,
        });
        match area {
            Area::Change => Message::Change(area::change::Message::Interaction(im)),
            Area::Caps => Message::Caps(area::caps::Message::Interaction(im)),
            Area::Codex => Message::Codex(area::codex::Message::Interaction(im)),
            Area::Ideas => Message::Ideas(area::ideas::Message::Interaction(im)),
            Area::Dashboard | Area::Settings => Message::Interaction(im),
        }
    })
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
            // Discriminate by *active area*, not by whether an idea happens to
            // point at this scope. An idea-promoted change has an
            // `idea_for_scope` hit but can be viewed from either area, and
            // `ideas::handle_interaction` keys off `state.ideas.selected` —
            // so routing to Ideas from the Change area silently drops the
            // message (cmd+N parity with the `+` button, which uses the
            // calling area's own wrap, depends on this).
            if state.active_area == Area::Ideas && state.ideas.idea_for_scope(key).is_some() {
                update(
                    state,
                    Message::Ideas(area::ideas::Message::Interaction(msg)),
                )
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
    duckspec_root: Option<&Path>,
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
        let format_result = idea_format::format_body(&payload.body, duckspec_root);
        let body_to_save: String = match &format_result {
            Ok(formatted) => formatted.clone(),
            Err(_) => payload.body.clone(),
        };
        if let Err(e) = idea_store::save_idea(&mut idea, &body_to_save, project_root) {
            tracing::warn!("quick idea save failed: {e}");
            return;
        }
        let new_path = idea.abs_path.clone();
        let new_title = idea.display_title();
        state.ideas.format_errors.remove(&loaded_path);
        state.ideas.format_errors.remove(&new_path);
        if let Err(errors) = &format_result {
            state
                .ideas
                .format_errors
                .insert(new_path.clone(), errors.clone());
        }
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
        let format_result = idea_format::format_body(&body, duckspec_root);
        let body_to_save: String = match &format_result {
            Ok(formatted) => formatted.clone(),
            Err(_) => body.clone(),
        };
        if let Err(e) = idea_store::save_idea(&mut idea, &body_to_save, project_root) {
            tracing::warn!("quick idea save failed: {e}");
            return;
        }
        if let Err(errors) = &format_result {
            state
                .ideas
                .format_errors
                .insert(idea.abs_path.clone(), errors.clone());
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

/// Default starting query for the new-file modal: the directory of the active
/// editor / diff tab's file, repo-relative, with a trailing `/`. Empty string
/// when no editor tab is open or its file lives outside the project root.
fn new_file_seed_path(state: &State) -> String {
    let Some(root) = state.project.project_root.as_deref() else {
        return String::new();
    };
    let Some(tab) = state.tabs.active_tab() else {
        return String::new();
    };
    let path = match &tab.view {
        tab_bar::TabView::Editor { path: Some(p), .. } => p.as_path(),
        tab_bar::TabView::Diff { path, .. } => path.as_path(),
        _ => return String::new(),
    };
    let Some(parent) = path.parent() else {
        return String::new();
    };
    let Ok(rel) = parent.strip_prefix(root) else {
        return String::new();
    };
    if rel.as_os_str().is_empty() {
        String::new()
    } else {
        format!("{}/", rel.display())
    }
}

/// Resolve the new-file modal's Enter action: create the file (touching parent
/// dirs as needed) if missing, then open it as a regular file tab. Existing
/// files take the same code path as the file finder's confirm.
fn confirm_new_file(state: &mut State, action: widget::new_file::ConfirmAction) -> Task<Message> {
    use widget::new_file::ConfirmAction;
    let abs = match &action {
        ConfirmAction::Open(p) | ConfirmAction::Create(p) => p.clone(),
    };
    if let ConfirmAction::Create(_) = &action {
        if let Some(parent) = abs.parent()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            tracing::warn!(path = %abs.display(), %e, "failed to create parent dir");
            return Task::none();
        }
        if let Err(e) = std::fs::File::create(&abs) {
            tracing::warn!(path = %abs.display(), %e, "failed to create file");
            return Task::none();
        }
        tracing::info!(path = %abs.display(), "created new file");
    }

    let Some(root) = state.project.project_root.clone() else {
        return Task::none();
    };
    let rel = abs.strip_prefix(&root).unwrap_or(&abs).to_path_buf();
    let content = std::fs::read_to_string(&abs).unwrap_or_default();
    let id = format!("file:{}", rel.display());
    let title = rel
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| rel.display().to_string());
    let area = match state.active_area {
        Area::Dashboard | Area::Settings => Area::Change,
        other => other,
    };
    let switched = state.active_area != area;
    switch_area(state, area);
    state
        .tabs
        .open_file(id.clone(), title, content, Some(abs.clone()));
    let mut task = Task::none();
    if let Some(tab) = state.tabs.file_tabs.iter_mut().find(|t| t.id == id)
        && let tab_bar::TabView::Editor { editor, .. } = &mut tab.view
    {
        task = spawn_file_tab_highlight(area, id, editor, state.highlighter.clone(), false);
    }
    if switched {
        Task::batch([task, restore_chat_scroll(state)])
    } else {
        task
    }
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
        Message::NewFile(_) => true,
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
    use widget::agent_chat::Msg as ChatMsg;
    matches!(
        msg,
        Some(
            interaction::Msg::NewSession
                | interaction::Msg::ClearSession
                // Empty Enter / send remounts input state; restore caret like Tab cycle.
                | interaction::Msg::AgentChat(
                    ChatMsg::SendPressed
                        | ChatMsg::ActivateFastResponse(_)
                        | ChatMsg::CycleNextAction(_)
                )
        )
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
                editor.highlight_spans = Some(widget::diff_view::build_diff_spans(diff_data, None));
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
                editor.highlight_spans =
                    Some(state.highlighter.highlight_lines(&editor.lines, md_syntax));
            }
        }
    }

    Task::batch(tasks)
}

/// Parse a `ToolUse` event into the change-folder slug it will create, or
/// `None` when the call is not a shell command running `ds create change`.
///
/// Attribution is content-first: any tool whose JSON input carries a
/// `command` string is eligible — Claude's `"Bash"` and grok's
/// `"run_terminal_command"` share that shape. The tool name is ignored so a
/// new harness does not silently drop bindings. The extracted argument is
/// slugified with the shared rule so the result equals the directory the CLI
/// creates. Anything unrecognized yields `None`, which declines to bind
/// rather than risk mis-attributing.
fn parse_create_change(input: &str) -> Option<String> {
    let command = serde_json::from_str::<serde_json::Value>(input)
        .ok()?
        .get("command")?
        .as_str()?
        .to_string();
    let arg = extract_create_change_arg(&command)?;
    let slug = duckpond::slug::slugify(&arg);
    (!slug.is_empty()).then_some(slug)
}

/// Locate a `ds create change` invocation in a shell command line and return
/// its argument. Takes the next shell token after `change`, honoring a single
/// quoted (single- or double-) multi-word argument, and stops at a shell
/// separator (`&&`, `;`, `|`, newline). Returns `None` when the invocation is
/// absent or has no argument.
fn extract_create_change_arg(command: &str) -> Option<String> {
    let marker = "ds create change";
    let start = command.find(marker)? + marker.len();
    let rest = command[start..].trim_start();

    // A quoted argument runs to its closing quote, spaces included.
    let mut chars = rest.chars();
    if let Some(quote @ ('"' | '\'')) = chars.clone().next() {
        chars.next();
        let arg: String = chars.take_while(|&c| c != quote).collect();
        return (!arg.is_empty()).then_some(arg);
    }

    // Otherwise the argument is the first whitespace-delimited token, cut short
    // by a shell separator so `… change foo && …` yields just `foo`.
    let token: String = rest
        .chars()
        .take_while(|&c| !c.is_whitespace() && c != '&' && c != ';' && c != '|')
        .collect();
    (!token.is_empty()).then_some(token)
}

/// Promote the exploration `exp_id` into the real change `new_name`, choosing
/// the correct promotion by whether the exploration is idea-owned. This is the
/// single dispatch point shared by binding-driven and fallback attribution, so
/// the idea-vs-change decision lives in exactly one place.
fn route_promotion(state: &mut State, exp_id: &str, new_name: &str) {
    let root = state.project.project_root.clone();
    let idea_path = state
        .change
        .explorations
        .iter()
        .find(|e| e.id == exp_id)
        .and_then(|e| e.idea_path.clone());

    match idea_path {
        None => area::change::promote_exploration(
            &mut state.change,
            &mut state.interactions,
            exp_id,
            new_name,
            root.as_deref(),
        ),
        Some(p) => {
            promote_idea_exploration(state, Path::new(&p), new_name);
            state.change.explorations.retain(|e| e.id != exp_id);
            chat_store::save_explorations(
                &state.change.explorations,
                state.change.exploration_counter,
                root.as_deref(),
            );
        }
    }
}

/// Promote the exploration bound to a newly-detected change directory, if any.
///
/// The binding recorded when an exploration session's agent ran
/// `ds create change <name>` (`pending_bindings`) is the sole authority for
/// attribution, and it is *consumed* here. A change with no binding — an
/// out-of-band creation, an unarchive, or a version-control reappearance of a
/// directory that already existed — is left standalone: UI focus never
/// attributes a change to an unrelated exploration. Because the binding is
/// consumed, a later re-detection of the same directory finds none and does
/// not promote again.
///
/// Returns `true` when a binding was consumed and promotion ran (callers
/// should re-focus the chat input after the scope remount).
fn promote_bound_exploration(state: &mut State, new_name: &str) -> bool {
    if let Some(exploration_id) = state.change.pending_bindings.remove(new_name) {
        tracing::info!(
            from = exploration_id,
            to = new_name,
            "promoting exploration to real change"
        );
        route_promotion(state, &exploration_id, new_name);
        true
    } else {
        false
    }
}

/// Result of reloading project data and reconciling local UI state.
struct ReconcileOutcome {
    /// Tab IDs were rewritten for an external archival — refresh open tabs.
    archived: bool,
    /// A bound exploration was promoted into a new change — re-focus chat.
    promoted: bool,
}

/// Reload `ProjectData` and reconcile duckboard-local state: promote a selected
/// exploration if a new change appeared, migrate subscriptions when a change
/// was archived externally, and refresh lifecycle next-command / oneshot chips.
fn reload_and_reconcile(state: &mut State) -> ReconcileOutcome {
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

    // Detect a new change directory and promote the exploration that created
    // it — but only on the authoritative `ds create change` binding. An unbound
    // new directory (out-of-band creation, unarchive, VCS reappearance) is left
    // standalone; focus never attributes it.
    let mut promoted = false;
    if let Some(new_name) = state
        .project
        .active_changes
        .iter()
        .find(|c| !old_change_names.contains(&c.name))
        .map(|c| c.name.clone())
    {
        promoted = promote_bound_exploration(state, &new_name);
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

    let moves = idea_store::reconcile(&mut state.ideas.ideas, &state.project);
    for mv in moves {
        area::ideas::refresh_after_move(
            &mut state.ideas,
            &mut state.tabs,
            &mv.old_path,
            &mv.new_path,
            &mv.title,
        );
    }

    let dirty = !state.change.changed_files.is_empty();
    area::change::refresh_fast_response(
        &mut state.interactions,
        &state.project,
        state.config.chat.agent_input_hints,
        dirty,
    );
    ReconcileOutcome {
        archived: archived_any,
        promoted,
    }
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

    // Flush-before-mutate: persist every session before the exploration's
    // in-memory state is migrated, so an in-flight turn can't be lost.
    if let Some(ix) = state
        .interactions
        .get(&scope::Scope::Exploration(exploration_id.clone()))
    {
        interaction::flush_sessions(ix, project_root.as_deref());
    }
    if let Some(mut ix) = state
        .interactions
        .remove(&scope::Scope::Exploration(exploration_id.clone()))
    {
        for ax in ix.sessions.iter_mut() {
            ax.session.scope = change_name.to_string();
            ax.scope_kind = scope::ScopeKind::Change;
        }
        let target = scope::Scope::Change(change_name.to_string());
        if let Some(existing) = state.interactions.get_mut(&target) {
            // Target scope is already live — fold the exploration's sessions in
            // rather than overwrite, preserving the target's subscriptions.
            interaction::merge_sessions(existing, ix.sessions, change_name);
        } else {
            interaction::reconcile_display_names(&mut ix.sessions, change_name);
            state.interactions.insert(target, ix);
        }
    }
    chat_store::merge_scope(&exploration_id, change_name, project_root.as_deref());
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
) -> Option<(&'a mut widget::text_edit::EditorState, Arc<vcs::DiffData>)> {
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
/// Re-walk the project tree for the Files explorer. Only invoked while the
/// explorer section is expanded, so collapsed sessions never pay for the
/// walk.
fn refresh_project_files(state: &mut State) {
    let Some(root) = state.project.project_root.as_deref() else {
        return;
    };
    let mut files = Vec::new();
    widget::file_finder::walk_project_files(root, |rel| files.push(rel.to_path_buf()));
    state.change.set_project_files(&files);
}

/// If the Files explorer is expanded and the active tab is a `file:` tab for
/// a path inside the project, expand the file's ancestor directories and
/// scroll its row into view. Row highlighting itself derives from the active
/// tab id, so no selection state needs updating here.
fn reveal_active_file_in_explorer(state: &mut State) -> Task<Message> {
    if !state
        .change
        .expanded_sections
        .contains(area::change::FILES_SECTION)
    {
        return Task::none();
    }
    let Some(tab_id) = state.tabs.active_tab().map(|t| t.id.clone()) else {
        return Task::none();
    };
    let Some(rel) = tab_id.strip_prefix("file:") else {
        return Task::none();
    };
    if std::path::Path::new(rel).is_absolute() {
        // Files outside the project root never appear in the explorer.
        return Task::none();
    }
    state.change.expand_explorer_ancestors(rel);
    // A miss usually means the file was just created and the watcher hasn't
    // refreshed the tree yet — re-walk once before giving up.
    let position = state.change.explorer_flat_position(&tab_id).or_else(|| {
        refresh_project_files(state);
        state.change.explorer_flat_position(&tab_id)
    });
    let Some((index, count)) = position else {
        return Task::none();
    };
    widget::vertical_scroll::reveal_row(
        area::change::EXPLORER_VIEWPORT_ID,
        area::change::EXPLORER_CONTENT_ID,
        index,
        count,
        state.change.list_scroll,
    )
    .map(|offset| Message::Change(area::change::Message::ScrollList(offset)))
}

fn refresh_changed_files(state: &mut State) {
    if let Some(root) = &state.project.project_root {
        state.change.set_changed_files(vcs::changed_files(root));
    }
    // Commit chrome depends on dirty; recompose when the file list updates.
    let dirty = !state.change.changed_files.is_empty();
    area::change::refresh_fast_response(
        &mut state.interactions,
        &state.project,
        state.config.chat.agent_input_hints,
        dirty,
    );
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

// ── Focused-column tracking ────────────────────────────────────────────────

/// Heuristic: every editor-targeted message that implies focus pulls
/// `focused_column` toward that side. Used by cmd-f to pick a target.
/// Conservative — leaves the value untouched for messages that don't
/// involve a specific column (e.g. background events, area switches).
fn update_focused_column(state: &mut State, message: &Message) {
    use widget::text_edit::EditorAction;
    match message {
        Message::TabContent(tab_bar::TabContentMsg::EditorAction(_))
        | Message::TabContent(tab_bar::TabContentMsg::SearchSliceAction(_, _))
        | Message::TabSelect(_) => {
            state.focused_column = Some(FocusedColumn::Content);
        }
        Message::Interaction(interaction::Msg::AgentChat(inner)) => {
            // Any agent-chat editor action (input or block) signals chat
            // focus. Other agent_chat messages (scroll, completion popup
            // toggles) also count — they only fire from chat interactions.
            use widget::agent_chat::Msg as ChatMsg;
            let is_focus_signal = matches!(
                inner,
                ChatMsg::InputAction(_)
                    | ChatMsg::ChatAction(_, _)
                    | ChatMsg::QueueAction(_)
                    | ChatMsg::SendPressed
                    | ChatMsg::ActivateFastResponse(_)
                    | ChatMsg::ChatScrolled(_)
                    | ChatMsg::CycleNextAction(_)
            );
            // Click on a chat block or chat input → focus chat. Avoid
            // pulling focus on irrelevant variants (e.g. completion popup
            // toggles fired from elsewhere).
            if is_focus_signal {
                let pull = match inner {
                    ChatMsg::ChatAction(_, action)
                    | ChatMsg::InputAction(action)
                    | ChatMsg::QueueAction(action) => matches!(
                        action,
                        EditorAction::Click(_)
                            | EditorAction::Drag(_)
                            | EditorAction::DragEnd
                            | EditorAction::Insert(_)
                            | EditorAction::Paste(_)
                            | EditorAction::Backspace
                            | EditorAction::Delete
                            | EditorAction::Enter
                            | EditorAction::SelectAll
                    ),
                    _ => true,
                };
                if pull {
                    state.focused_column = Some(FocusedColumn::Chat);
                }
            }
        }
        _ => {}
    }
}

// ── Local find helpers ─────────────────────────────────────────────────────

/// Build the modal snapshot for the focused target. The snapshot lets the
/// modal compute live previews without holding borrows on the app state.
fn build_find_snapshot(
    state: &State,
) -> Option<(widget::find::FindTarget, widget::find::ModalSnapshot)> {
    let target = keybinds::keybind_find(state)?;
    match &target {
        widget::find::FindTarget::Editor(tab_id) => {
            let tab = state.tabs.active_tab()?;
            if &tab.id != tab_id {
                return None;
            }
            let editor = match &tab.view {
                tab_bar::TabView::Editor { editor, .. } | tab_bar::TabView::Diff { editor, .. } => {
                    editor
                }
                _ => return None,
            };
            let label =
                widget::find::editor_label_for(tab_id, state.project.project_root.as_deref());
            let snap = widget::find::snapshot_editor(target.clone(), label, editor);
            Some((target, snap))
        }
        widget::find::FindTarget::ChatSession(_, session_id) => {
            let scope = state.active_scope()?;
            let ix = state.interactions.get(&scope)?;
            let ax = ix.active()?;
            if &ax.session.id != session_id {
                return None;
            }
            let roles: Vec<&'static str> = ax
                .chat_blocks
                .iter()
                .map(|b| chat_block_role_label(b.kind))
                .collect();
            let searchable = chat_block_searchable(&ax.chat_blocks);
            let label = ax.session.display_name.clone();
            let snap = widget::find::snapshot_chat(
                target.clone(),
                label,
                &ax.chat_editors,
                &roles,
                &searchable,
            );
            Some((target, snap))
        }
    }
}

/// Role label for chat find / selection context chips.
fn chat_block_role_label(kind: widget::text_edit::BlockKind) -> &'static str {
    match kind {
        widget::text_edit::BlockKind::User => "User",
        widget::text_edit::BlockKind::Assistant => "Assistant",
        widget::text_edit::BlockKind::Reasoning => "Thinking",
        widget::text_edit::BlockKind::Activity => "Activity",
        widget::text_edit::BlockKind::ToolUse => "Tool",
        widget::text_edit::BlockKind::ToolResult => "Result",
        widget::text_edit::BlockKind::System => "System",
    }
}

/// Predicate vector mirroring `chat_blocks`: `true` for conversation prose
/// (user/answer/thinking/system), `false` for tool plumbing. Local find
/// stays scoped to the conversation surface.
fn chat_block_searchable(blocks: &[widget::text_edit::Block]) -> Vec<bool> {
    blocks
        .iter()
        .map(|b| {
            !matches!(
                b.kind,
                widget::text_edit::BlockKind::Activity
                    | widget::text_edit::BlockKind::ToolUse
                    | widget::text_edit::BlockKind::ToolResult
            )
        })
        .collect()
}

/// Open the find modal for the focused column. Clears any prior find for
/// the same target so the modal always enters fresh "create" mode (per the
/// design contract: cmd-f is the gesture to start over).
fn open_find_modal(state: &mut State) -> Task<Message> {
    let Some((target, snapshot)) = build_find_snapshot(state) else {
        return Task::none();
    };
    state.find_states.remove(&target);
    state.find_modal.open(snapshot);
    // Match the other modals' behaviour: release the terminal focus latch
    // so the PTY doesn't swallow the user's typing into the find input.
    for ix in state.interactions.values_mut() {
        ix.terminal_focused = false;
    }
    iced::widget::operation::focus(widget::find::FIND_INPUT_ID)
}

/// Apply a find::Msg to the app. The modal owns the live preview state;
/// committing creates a `FindState` keyed by target that survives modal
/// dismissal and tab switches.
fn handle_find_msg(state: &mut State, msg: widget::find::Msg) -> Task<Message> {
    use widget::find::Msg;
    match msg {
        Msg::QueryChanged(q) => {
            state.find_modal.set_query(q);
            Task::none()
        }
        Msg::PreviewSelectNext => {
            state.find_modal.select_next();
            Task::none()
        }
        Msg::PreviewSelectPrev => {
            state.find_modal.select_prev();
            Task::none()
        }
        Msg::Cancel => {
            // Esc / click-away: close the modal AND clear any active find
            // for the modal's current target. Cmd-f always enters fresh.
            if let Some(target) = state.find_modal.target().cloned() {
                state.find_states.remove(&target);
            }
            state.find_modal.close();
            Task::none()
        }
        Msg::Commit => commit_find(state),
        Msg::Navigate(target, dir) => navigate_find(state, &target, dir),
        Msg::Deactivate(target) => {
            state.find_states.remove(&target);
            Task::none()
        }
    }
}

fn commit_find(state: &mut State) -> Task<Message> {
    let Some(target) = state.find_modal.target().cloned() else {
        return Task::none();
    };
    let query = state.find_modal.query.clone();
    if query.is_empty() {
        // Empty query: dismiss without activating, don't store an empty find.
        state.find_modal.close();
        return Task::none();
    }
    let matches = match &target {
        widget::find::FindTarget::Editor(tab_id) => {
            let Some(tab) = state.tabs.active_tab() else {
                state.find_modal.close();
                return Task::none();
            };
            if &tab.id != tab_id {
                state.find_modal.close();
                return Task::none();
            }
            let editor = match &tab.view {
                tab_bar::TabView::Editor { editor, .. } | tab_bar::TabView::Diff { editor, .. } => {
                    editor
                }
                _ => {
                    state.find_modal.close();
                    return Task::none();
                }
            };
            match widget::find::matches_for_editor(&query, editor) {
                Ok(m) => m,
                Err(_) => return Task::none(),
            }
        }
        widget::find::FindTarget::ChatSession(_, session_id) => {
            let Some(scope) = state.active_scope() else {
                state.find_modal.close();
                return Task::none();
            };
            let Some(ix) = state.interactions.get(&scope) else {
                state.find_modal.close();
                return Task::none();
            };
            let Some(ax) = ix.active() else {
                state.find_modal.close();
                return Task::none();
            };
            if &ax.session.id != session_id {
                state.find_modal.close();
                return Task::none();
            }
            let roles: Vec<&'static str> = ax
                .chat_blocks
                .iter()
                .map(|b| chat_block_role_label(b.kind))
                .collect();
            let searchable = chat_block_searchable(&ax.chat_blocks);
            match widget::find::matches_for_chat(&query, &ax.chat_editors, &roles, &searchable) {
                Ok(m) => m,
                Err(_) => return Task::none(),
            }
        }
    };
    state.find_modal.close();
    if matches.is_empty() {
        // No matches: don't bother activating the toolbar.
        return Task::none();
    }
    let find_state = widget::find::FindState {
        query,
        matches,
        current: 0,
    };
    state.find_states.insert(target.clone(), find_state);
    jump_to_current(state, &target)
}

fn navigate_find(
    state: &mut State,
    target: &widget::find::FindTarget,
    dir: widget::find::NavDir,
) -> Task<Message> {
    if let Some(fs) = state.find_states.get_mut(target) {
        match dir {
            widget::find::NavDir::Next => fs.select_next(),
            widget::find::NavDir::Prev => fs.select_prev(),
        }
    }
    jump_to_current(state, target)
}

/// ⌘↑/↓/←/→ chat landmarks — history ends and Answer-to-Answer jumps.
///
/// Every arm sets `chat_scroll_overridden` so `update_with_scroll_preservation`
/// does not replay a pre-key snapshot over the jump. Leave-bottom jumps clear
/// stick so StreamTick auto-snap cannot undo them while streaming.
fn apply_chat_landmark(
    state: &mut State,
    action: keybinds::ChatLandmarkAction,
) -> Task<Message> {
    use keybinds::ChatLandmarkAction;
    use widget::agent_chat;

    // Always win against scroll-preservation replay for this tick.
    state.chat_scroll_overridden = true;

    match action {
        ChatLandmarkAction::HistoryTop => {
            if let Some(scope) = state.active_scope()
                && let Some(ix) = state.interactions.get_mut(&scope)
                && let Some(ax) = ix.active_mut()
            {
                ax.stick_to_bottom = false;
                ax.pending_snap_to_bottom = false;
                ax.last_chat_offset_y = Some(0.0);
            }
            iced::widget::operation::scroll_to(
                agent_chat::CHAT_SCROLLABLE_ID,
                iced::widget::scrollable::AbsoluteOffset { x: 0.0, y: 0.0 },
            )
        }
        ChatLandmarkAction::HistoryBottom => {
            if let Some(scope) = state.active_scope()
                && let Some(ix) = state.interactions.get_mut(&scope)
                && let Some(ax) = ix.active_mut()
            {
                ax.stick_to_bottom = true;
                ax.pending_snap_to_bottom = false;
            }
            iced::widget::operation::snap_to_end(agent_chat::CHAT_SCROLLABLE_ID)
        }
        ChatLandmarkAction::PrevAnswer | ChatLandmarkAction::NextAnswer => {
            let go_prev = matches!(action, ChatLandmarkAction::PrevAnswer);
            let Some(scope) = state.active_scope() else {
                return Task::none();
            };
            let (anchors, offset_y, stick) = {
                let State {
                    interactions,
                    highlighter,
                    ..
                } = state;
                let Some(ix) = interactions.get_mut(&scope) else {
                    return Task::none();
                };
                let Some(ax) = ix.active_mut() else {
                    return Task::none();
                };
                // Mid-stream with stick off defers materialize — paint blocks so
                // Answer anchors and widget ids exist before the layout Operation.
                if ax.chat_ui_dirty {
                    interaction::materialize_chat_ui(ax, highlighter);
                }
                let anchors = agent_chat::answer_block_indices(&ax.chat_blocks);
                if anchors.is_empty() {
                    return Task::none();
                }
                let offset_y = ax.last_chat_offset_y.unwrap_or(0.0);
                let stick = ax.stick_to_bottom;
                // Leave-bottom jump: unstick so StreamTick auto-snap does not fight us.
                ax.stick_to_bottom = false;
                ax.pending_snap_to_bottom = false;
                (anchors, offset_y, stick)
            };
            agent_chat::scroll_to_adjacent_answer(&anchors, go_prev, offset_y, stick)
        }
    }
}

/// Move the cursor / scroll position so the current match is visible.
/// Editor: set cursor at the match end, scroll to bring the line into
/// view, and focus the editor so the cursor paints. Chat: estimate the
/// y offset of the matching block + line and scroll the chat area there.
/// Also clears `stick_to_bottom` so streaming auto-snap doesn't fight us.
fn jump_to_current(state: &mut State, target: &widget::find::FindTarget) -> Task<Message> {
    let Some(fs) = state.find_states.get(target) else {
        return Task::none();
    };
    let Some(m) = fs.current_match().cloned() else {
        return Task::none();
    };
    match target {
        widget::find::FindTarget::Editor(tab_id) => {
            let Some(tab) = state.tabs.active_tab_mut() else {
                return Task::none();
            };
            if &tab.id != tab_id {
                return Task::none();
            }
            let editor = match &mut tab.view {
                tab_bar::TabView::Editor { editor, .. } | tab_bar::TabView::Diff { editor, .. } => {
                    editor
                }
                _ => return Task::none(),
            };
            let line = m.line.min(editor.lines.len().saturating_sub(1));
            let line_len = editor.lines[line].len();
            let col = m.byte_end.min(line_len);
            editor.cursor = widget::text_edit::Pos::new(line, col);
            editor.anchor = None;
            // Center-ish: line * LINE_HEIGHT - 1/3 of viewport. We don't
            // know viewport height here so use a fixed 200px window.
            let target_y = line as f32 * 20.0 - 200.0;
            editor.scroll_y = target_y.max(0.0);
            // Focus the editor so its `InternalState::focused` flips on —
            // the renderer paints the caret only when focused, so without
            // this the cursor we just placed would be invisible.
            iced::widget::operation::focus(tab_bar::editor_focus_id(tab_id))
        }
        widget::find::FindTarget::ChatSession(_, _) => {
            let Some(block_idx) = m.block_idx else {
                return Task::none();
            };
            let Some(scope) = state.active_scope() else {
                return Task::none();
            };
            // Tell the scroll-preservation wrapper to skip its replay this
            // tick — otherwise our scroll_to gets overwritten with the
            // pre-keypress offset.
            state.chat_scroll_overridden = true;
            // Drop stick-to-bottom so a streaming auto-snap can't override
            // the scroll we're about to issue.
            if let Some(ix) = state.interactions.get_mut(&scope)
                && let Some(ax) = ix.active_mut()
            {
                ax.stick_to_bottom = false;
                ax.pending_snap_to_bottom = false;
            }
            // Scroll the matching block's container to the top of the
            // chat scrollable. The Operation reads the actual laid-out
            // bounds, so word-wrap / collapsed tools / per-kind padding
            // all come out exact — no pixel math from outside the layout
            // pass.
            widget::find::scroll_block_to_top(
                widget::agent_chat::CHAT_SCROLLABLE_ID,
                widget::find::chat_block_widget_id(block_idx),
            )
        }
    }
}

/// Build the highlight ranges for the editor in the active tab — both the
/// "all matches" set and the current candidate. Returns empty when no find
/// is active for that tab.
fn editor_find_highlights(
    state: &State,
    tab_id: &str,
) -> (
    Vec<widget::text_edit::HighlightRange>,
    Option<widget::text_edit::HighlightRange>,
) {
    let target = widget::find::FindTarget::editor(tab_id.to_string());
    let Some(fs) = state.find_states.get(&target) else {
        return (Vec::new(), None);
    };
    let ranges: Vec<widget::text_edit::HighlightRange> = fs
        .matches
        .iter()
        .filter(|m| m.block_idx.is_none())
        .map(|m| widget::text_edit::HighlightRange {
            line: m.line,
            byte_start: m.byte_start,
            byte_end: m.byte_end,
        })
        .collect();
    let current = fs
        .current_match()
        .filter(|m| m.block_idx.is_none())
        .map(|m| widget::text_edit::HighlightRange {
            line: m.line,
            byte_start: m.byte_start,
            byte_end: m.byte_end,
        });
    (ranges, current)
}

/// Build per-block highlight ranges for a chat session. Returns a Vec
/// indexed by block_idx; each entry is the (ranges, current) pair for that
/// block. Empty Vec when no find is active for the session.
fn chat_find_highlights(
    state: &State,
    instance_id: u64,
    session_id: &str,
    block_count: usize,
) -> Vec<(
    Vec<widget::text_edit::HighlightRange>,
    Option<widget::text_edit::HighlightRange>,
)> {
    let target = widget::find::FindTarget::chat(instance_id, session_id.to_string());
    let Some(fs) = state.find_states.get(&target) else {
        return Vec::new();
    };
    let mut out: Vec<(
        Vec<widget::text_edit::HighlightRange>,
        Option<widget::text_edit::HighlightRange>,
    )> = (0..block_count).map(|_| (Vec::new(), None)).collect();
    let current_idx = fs.current;
    for (i, m) in fs.matches.iter().enumerate() {
        let Some(bi) = m.block_idx else { continue };
        if bi >= out.len() {
            continue;
        }
        let range = widget::text_edit::HighlightRange {
            line: m.line,
            byte_start: m.byte_start,
            byte_end: m.byte_end,
        };
        out[bi].0.push(range);
        if i == current_idx {
            out[bi].1 = Some(range);
        }
    }
    out
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

    state.tabs.preview = state.cached_previews.remove(&target).unwrap_or(None);
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
            state.tabs.active = if state.tabs.preview.is_some() || state.tabs.file_tabs.is_empty() {
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
                state.config.chat.agent_input_hints,
                        state.window_width,
                );
        }
        Area::Caps => {
            let ix = state.interactions.entry(scope::Scope::Caps).or_default();
            area::caps::update(
                &mut state.caps,
                &mut state.tabs,
                ix,
                area::caps::Message::Interaction(msg),
                &state.project,
                &state.highlighter,
                state.config.chat.agent_input_hints,
                        state.window_width,
                );
        }
        Area::Codex => {
            let ix = state.interactions.entry(scope::Scope::Codex).or_default();
            area::codex::update(
                &mut state.codex,
                &mut state.tabs,
                ix,
                area::codex::Message::Interaction(msg),
                &state.project,
                &state.highlighter,
                state.config.chat.agent_input_hints,
                        state.window_width,
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
                state.config.chat.agent_input_hints,
                        state.window_width,
                );
        }
        Area::Dashboard | Area::Settings => {}
    }
    if needs_focus {
        focus_chat_input()
    } else {
        Task::none()
    }
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
/// Pull a cmd-clicked path reference out of any message route that carries
/// editor or terminal actions. Returns `(path, 1-based line)`.
fn extract_open_path(msg: &Message) -> Option<(String, Option<usize>)> {
    fn from_action(action: &widget::text_edit::EditorAction) -> Option<(String, Option<usize>)> {
        if let widget::text_edit::EditorAction::OpenPath { path, line } = action {
            Some((path.clone(), *line))
        } else {
            None
        }
    }
    fn from_interaction(im: &interaction::Msg) -> Option<(String, Option<usize>)> {
        match im {
            interaction::Msg::TerminalOpenPath { path, line } => Some((path.clone(), *line)),
            interaction::Msg::AgentChat(
                widget::agent_chat::Msg::InputAction(action)
                | widget::agent_chat::Msg::ChatAction(_, action)
                | widget::agent_chat::Msg::QueueAction(action),
            ) => from_action(action),
            _ => None,
        }
    }
    match msg {
        Message::TabContent(tab_bar::TabContentMsg::EditorAction(action)) => from_action(action),
        Message::TabContent(tab_bar::TabContentMsg::SearchSliceAction(_, action)) => {
            from_action(action)
        }
        Message::QuickIdea(widget::quick_idea::Msg::EditorAction(action)) => from_action(action),
        Message::Interaction(im) => from_interaction(im),
        Message::Caps(area::caps::Message::Interaction(im)) => from_interaction(im),
        Message::Codex(area::codex::Message::Interaction(im)) => from_interaction(im),
        Message::Ideas(area::ideas::Message::Interaction(im)) => from_interaction(im),
        Message::Change(area::change::Message::Interaction(im)) => from_interaction(im),
        _ => None,
    }
}

/// Handle a cmd-clicked path reference: open the file directly when it
/// resolves, otherwise fall back to the fuzzy file finder pre-filled with
/// the path. `line` is 1-based (from a `:NN` suffix in the text).
fn open_path_reference(state: &mut State, path: &str, line: Option<usize>) -> Task<Message> {
    let line_idx = line.map(|l| l.saturating_sub(1));
    if let Some(abs) = path_link::resolve(path) {
        return open_path_in_tab(state, abs, line_idx);
    }
    let Some(root) = state.project.project_root.clone() else {
        return Task::none();
    };
    state.file_finder.open(&root);
    state.file_finder.set_query(path.to_string());
    state.file_finder.pending_line = line_idx;
    for ix in state.interactions.values_mut() {
        ix.terminal_focused = false;
    }
    iced::widget::operation::focus("file-finder-input")
}

/// Open `abs` as a file tab, switching to a tab-capable area when needed,
/// and optionally jump to a 0-based line. Shared by the file finder and
/// cmd-clicked path references.
fn open_path_in_tab(state: &mut State, abs: PathBuf, line: Option<usize>) -> Task<Message> {
    let Ok(content) = std::fs::read_to_string(&abs) else {
        return Task::none();
    };
    // Files inside the project keep root-relative tab ids (matching every
    // other open path); files outside fall back to the absolute path.
    let display = state
        .project
        .project_root
        .as_deref()
        .and_then(|root| abs.strip_prefix(root).ok())
        .unwrap_or(&abs)
        .to_path_buf();
    let id = format!("file:{}", display.display());
    let title = display
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| display.display().to_string());
    let area = match state.active_area {
        Area::Dashboard | Area::Settings => Area::Change,
        other => other,
    };
    let switched = state.active_area != area;
    switch_area(state, area);
    state
        .tabs
        .open_file(id.clone(), title, content, Some(abs.clone()));
    let mut task = Task::none();
    if let Some(tab) = state.tabs.file_tabs.iter_mut().find(|t| t.id == id)
        && let tab_bar::TabView::Editor { editor, .. } = &mut tab.view
    {
        if let Some(line) = line {
            let line = line.min(editor.lines.len().saturating_sub(1));
            editor.cursor = widget::text_edit::Pos::new(line, 0);
            editor.scroll_y = (line as f32 * 20.0 - 300.0).max(0.0);
        }
        task = spawn_file_tab_highlight(area, id, editor, state.highlighter.clone(), false);
    }
    if switched {
        Task::batch([task, restore_chat_scroll(state)])
    } else {
        task
    }
}

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
    let list: Element<'_, Message> =
        match state.active_area {
            Area::Change => {
                area::change::view_list(&state.change, &state.project, &state.ideas, &state.tabs)
                    .map(Message::Change)
            }
            Area::Caps => {
                area::caps::view_list(&state.caps, &state.project, &state.tabs).map(Message::Caps)
            }
            Area::Codex => area::codex::view_list(&state.codex, &state.project, &state.tabs)
                .map(Message::Codex),
            Area::Ideas => area::ideas::view_list(&state.ideas, &state.project, &state.tabs)
                .map(Message::Ideas),
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

    // Exploration without tabs still omits the door handle so the empty-state
    // instructions dominate; content hide is tab-based for every area.
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

    let content_collapsed = ix.is_some_and(|i| i.content_collapsed);
    // Content shows only when there is at least one open tab and the door has
    // not collapsed it. No tabs → hide empty shell; chat fills free space.
    let show_content = interaction::show_content_column(has_tabs, content_collapsed);

    if show_content {
        row_items = row_items.push(container(content).width(Length::Fill).height(Length::Fill));
    }

    let visible = ix.is_some_and(|i| i.visible);
    let width = ix.map_or(
        interaction::equal_interaction_width(state.window_width),
        |i| i.width,
    );
    let free_max = interaction::free_content_chat_width(state.window_width);

    // Door handle: always when tabs exist; for non-exploration also when the
    // panel is available without tabs (so chat can open/close while content is hidden).
    if !is_exploration || has_tabs {
        let toggle =
            widget::interaction_toggle::view(visible, content_collapsed, width, free_max, |m| {
                Message::Interaction(interaction::Msg::Handle(m))
            });
        row_items = row_items.push(toggle);
    }

    if let Some(ix) = ix
        && ix.visible
    {
        // Per-block highlights for the active session's chat editors.
        let block_hl: Vec<(
            Vec<widget::text_edit::HighlightRange>,
            Option<widget::text_edit::HighlightRange>,
        )> = if let Some(ax) = ix.active() {
            chat_find_highlights(state, ix.instance_id, &ax.session.id, ax.chat_blocks.len())
        } else {
            Vec::new()
        };
        // Find toolbar above the chat (when active for this session).
        let find_toolbar: Option<Element<'_, Message>> = ix.active().and_then(|ax| {
            let target = widget::find::FindTarget::chat(ix.instance_id, ax.session.id.clone());
            state
                .find_states
                .get(&target)
                .map(|fs| widget::find::view_toolbar(target.clone(), fs).map(Message::Find))
        });
        let interaction_col =
            interaction::view_column(
            ix,
            Message::Interaction,
            controls,
            block_hl,
            find_toolbar,
            state.config.chat.agent_input_hints,
        );
        let col = container(interaction_col)
            .height(Length::Fill)
            .style(theme::surface);
        let col = match interaction::interaction_column_size(show_content, ix.width) {
            interaction::InteractionColumnSize::Fixed(w) => col.width(w),
            interaction::InteractionColumnSize::Fill => col.width(Length::Fill),
        };
        row_items = row_items.push(col);
    }

    row_items.height(Length::Fill).into()
}

/// Render the shared content column: tab bar + (optional) area-specific
/// toolbar + tab content + (Change-only) error panel.
fn view_global_content(state: &State) -> Element<'_, Message> {
    let bar = tab_bar::view_bar(
        &state.tabs,
        Message::TabSelect,
        Message::TabClose,
        Message::TabArmClose,
        state.armed_tab_close,
    );

    // Editor find highlights + toolbar — only when the active tab has a
    // committed find for it.
    let editor_hl_owned = state
        .tabs
        .active_tab()
        .map(|t| editor_find_highlights(state, &t.id))
        .unwrap_or_else(|| (Vec::new(), None));
    let editor_hl = if editor_hl_owned.0.is_empty() && editor_hl_owned.1.is_none() {
        None
    } else {
        Some(editor_hl_owned)
    };
    let body = tab_bar::view_content(&state.tabs, editor_hl).map(Message::TabContent);

    let editor_toolbar: Option<Element<'_, Message>> = state.tabs.active_tab().and_then(|tab| {
        let target = widget::find::FindTarget::editor(tab.id.clone());
        state
            .find_states
            .get(&target)
            .map(|fs| widget::find::view_toolbar(target.clone(), fs).map(Message::Find))
    });

    let mut col = column![bar];

    // Idea pinned-tab toolbar (Explore / Open Change / Archive / Delete).
    if state.active_area == Area::Ideas
        && let Some(toolbar) = area::ideas::view_pinned_toolbar(&state.ideas, &state.tabs)
    {
        col = col.push(toolbar.map(Message::Ideas));
    }

    if let Some(toolbar) = editor_toolbar {
        col = col.push(toolbar);
    }

    col = col.push(body);

    // Change area: error panel for the active artifact.
    if state.active_area == Area::Change
        && let Some(errors) =
            area::change::error_panel_for(&state.change, &state.project, &state.tabs)
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
            &state.ideas.format_errors,
        )
        .map(Message::Dashboard),
        Area::Settings => area::settings::view(
            &state.settings,
            &state.config,
            state.project.project_root.as_deref(),
        )
        .map(Message::Settings),
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
    // Cmd-K hint surfaces in the status bar when there's a live selection
    // attachment on the active session: tells the user the kept-around
    // gesture is currently meaningful.
    let cmd_k_hint = state
        .active_scope()
        .and_then(|scope| state.interactions.get(&scope))
        .filter(|ix| ix.visible && ix.active_tab == ActiveTab::Chat)
        .and_then(|ix| ix.active())
        .filter(|ax| ax.selection_tentative.is_some())
        .map(|_| "⌘K keep selection".to_string());
    let status_bar = widget::status_bar::view(segments, project_label, cmd_k_hint);
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

    // Always render as a stack so the widget-tree shape stays the same when a
    // modal opens/closes. Otherwise the top-level type would flip between
    // `Column` and `Stack`, which restructures the subtree and resets the
    // chat scrollable's state to (0, 0) — see `restore_chat_scroll`.
    let overlay: Element<'_, Message> = if state.project_picker.visible {
        widget::project_picker::view(&state.project_picker, &state.config.projects.recent)
            .map(Message::ProjectPicker)
    } else if state.file_finder.visible {
        widget::file_finder::view(&state.file_finder).map(Message::FileFinder)
    } else if state.text_search.visible {
        widget::text_search::view(&state.text_search).map(Message::TextSearch)
    } else if state.quick_idea.visible {
        widget::quick_idea::view(&state.quick_idea).map(Message::QuickIdea)
    } else if state.new_file.visible {
        widget::new_file::view(&state.new_file).map(Message::NewFile)
    } else if state.find_modal.visible {
        widget::find::view_modal(&state.find_modal).map(Message::Find)
    } else {
        Space::new().width(0.0).height(0.0).into()
    };
    stack![main_view, overlay].into()
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
                // The worker runs on the provider named by the session's
                // resolved model harness (per-chat pin → project default →
                // built-in default). Folding it into the subscription respawns
                // the worker on the new backend when the harness changes.
                let harness = interaction::resolve_turn_model(
                    session.session.selected_model.as_ref(),
                    session.project_model_default.as_ref(),
                )
                .harness;
                let oneshot_model = agent::resolved_oneshot_model_for(
                    &harness,
                    state.config.chat.oneshot_model(&harness),
                );
                subs.push(
                    agent::agent_subscription(key, root.clone(), harness, oneshot_model)
                        .map(tagged_agent),
                );
            }
        };
        for ix in state.interactions.values() {
            push_scope(ix, &mut subs);
        }
    }

    // Global keyboard events.
    subs.push(event::listen_raw(handle_key_event));

    // One-shot process model catalog refresh + UI wake when ready.
    subs.push(model_catalog_ready_subscription());

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
        // Coalesced ~1s eager-persist tick, active only while streaming so idle
        // chats don't wake the runtime. Bounds mid-turn crash loss to ~1s.
        subs.push(
            iced::time::every(std::time::Duration::from_secs(1)).map(|_instant| Message::FlushTick),
        );
    }

    // Intercept window-close so we can flush every session before the window
    // goes away. Paired with `exit_on_close_request(false)` in `main`.
    subs.push(iced::window::close_requests().map(Message::WindowCloseRequested));

    // Equal content/chat split tracks free space; uncustomized panels rebalance.
    subs.push(
        iced::window::resize_events().map(|(_id, size)| Message::WindowResized(size)),
    );

    // ~60fps tick driving terminal edge auto-scroll. Only subscribed while a
    // terminal drag holds the pointer past an edge, so the render loop stays
    // idle otherwise.
    if any_terminal_autoscrolling(state) {
        subs.push(
            iced::time::every(std::time::Duration::from_millis(16))
                .map(|_| Message::TerminalAutoscrollTick),
        );
    }

    Subscription::batch(subs)
}

/// True if any terminal across all interaction panels is currently drag
/// auto-scrolling (drag active and pointer past a vertical edge).
fn any_terminal_autoscrolling(state: &State) -> bool {
    state
        .interactions
        .values()
        .any(|ix| ix.terminals.iter().any(|tt| tt.state.is_drag_autoscrolling()))
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

/// Refresh the process model catalog once per process, then emit
/// [`Message::ModelCatalogReady`] so views and agent subscriptions re-read it.
fn model_catalog_ready_subscription() -> Subscription<Message> {
    Subscription::run(model_catalog_ready_stream)
}

fn model_catalog_ready_stream() -> impl iced::futures::Stream<Item = Message> {
    use iced::futures::stream::{self, StreamExt};
    use std::sync::atomic::{AtomicBool, Ordering};
    static STARTED: AtomicBool = AtomicBool::new(false);

    stream::once(async {
        if !STARTED.swap(true, Ordering::SeqCst) {
            let _ = tokio::task::spawn_blocking(agent::refresh_model_catalog).await;
        }
        Message::ModelCatalogReady
    })
    .boxed()
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
        Ok(child) => {
            tracing::info!(pid = child.id(), exe = %exe.display(), "spawned new duckboard instance")
        }
        Err(e) => tracing::warn!(exe = %exe.display(), "spawn_new_instance: spawn failed: {e}"),
    }
}

fn main() -> iced::Result {
    // Must run before any threads are spawned (tracing, iced runtime, etc.).
    // When launched from Finder, launchd gives the .app bundle a stripped
    // PATH that misses every user-level install dir — any Command::new spawn
    // in the app would fail with ENOENT.
    path_env::augment();

    // Capture panics to a per-launch file. Stderr is detached when launched
    // from Finder, and Apple's crash reporter can't recover the panic message
    // from a `block2` `panic_cannot_unwind` abort — so we need our own log.
    install_panic_log();

    tracing_subscriber::fmt::init();

    // Detect system dark/light mode before creating the window.
    theme::set_mode(theme::detect_mode());
    tracing::info!(mode = ?theme::mode(), "duckboard starting");
    // Model catalog refresh runs once via subscription and wakes the UI with
    // Message::ModelCatalogReady (see `model_catalog_ready_subscription`).

    iced::application(State::new, update_with_scroll_preservation, view)
        .subscription(subscription)
        .title("duckboard")
        .theme(theme_fn)
        .window_size((theme::DEFAULT_WINDOW_WIDTH, 800.0))
        // We flush every chat session on close before letting the window go
        // away (see `Message::WindowCloseRequested`), so suppress the default
        // close-on-request behavior.
        .exit_on_close_request(false)
        .run()
}

fn theme_fn(_state: &State) -> iced::Theme {
    theme::app_theme()
}

/// Append every panic to `~/.config/duckboard/logs/panic-<utc-timestamp>.log`
/// so we have a record after the process aborts. The default panic handler
/// writes only to stderr, which is detached when the app is launched from
/// Finder; and `block2`'s FFI-boundary `panic_cannot_unwind` abort prevents
/// the OS crash reporter from capturing the original panic message.
fn install_panic_log() {
    let dir = config::config_dir().join("logs");
    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!("panic-log: failed to create {}: {e}", dir.display());
        return;
    }
    let stamp = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Iso8601::DEFAULT)
        .unwrap_or_else(|_| format!("{}", std::process::id()));
    let path = dir.join(format!("panic-{}.log", stamp.replace(':', "-")));

    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        use std::io::Write;
        let backtrace = std::backtrace::Backtrace::force_capture();
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            let _ = writeln!(
                f,
                "---- panic at {} ----\n{info}\n{backtrace}\n",
                time::OffsetDateTime::now_utc()
                    .format(&time::format_description::well_known::Iso8601::DEFAULT)
                    .unwrap_or_default(),
            );
        }
        default_hook(info);
    }));
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
            //
            // ⌘↑/↓/←/→ are also exempt: TextEdit captures them for caret /
            // document motion, but chat landmarks must still run with the
            // composer focused (and while a turn streams).
            let is_escape = matches!(&key, keyboard::Key::Named(keyboard::key::Named::Escape));
            let is_cmd_arrow_landmark = {
                use keyboard::key::Named;
                modifiers.command()
                    && !modifiers.shift()
                    && !modifiers.alt()
                    && matches!(
                        &key,
                        keyboard::Key::Named(
                            Named::ArrowUp
                                | Named::ArrowDown
                                | Named::ArrowLeft
                                | Named::ArrowRight
                        )
                    )
            };
            if !is_escape
                && !is_cmd_arrow_landmark
                && matches!(status, event::Status::Captured)
            {
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

/// Shared test harness for anything that resolves paths through
/// `config::data_dir` (chats, explorations, project data). `HOME` is a
/// process-global, and `std::env::set_var` races any concurrent reader — so
/// **every** test that mutates `HOME` must serialise through the single
/// `HOME_LOCK` here rather than a per-module lock, or two modules' tests can
/// set `HOME` at the same time.
#[cfg(test)]
pub(crate) mod test_support {
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static FS_COUNTER: AtomicU64 = AtomicU64::new(0);

    /// A unique temp directory, removed on drop.
    pub(crate) struct FsTmp(PathBuf);

    impl FsTmp {
        pub(crate) fn new() -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let counter = FS_COUNTER.fetch_add(1, Ordering::SeqCst);
            let mut p = std::env::temp_dir();
            p.push(format!("duckboard-test-{nanos}-{counter}"));
            std::fs::create_dir_all(&p).unwrap();
            Self(p)
        }
        pub(crate) fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for FsTmp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// The single lock guarding `HOME` mutation across all test modules.
    static HOME_LOCK: Mutex<()> = Mutex::new(());

    /// Run `f` with `HOME` set to `home` so `config::data_dir` resolves under
    /// it, restoring the previous value afterward. Serialised through
    /// `HOME_LOCK` so concurrent tests never race on the env var.
    pub(crate) fn with_home<R>(home: &Path, f: impl FnOnce() -> R) -> R {
        let _guard = HOME_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let prev = std::env::var_os("HOME");
        // SAFETY: tests serialise through HOME_LOCK so concurrent set_var is impossible.
        unsafe { std::env::set_var("HOME", home) };
        let out = f();
        // SAFETY: same lock guarantees no concurrent reader observing the
        // mutation race here.
        unsafe {
            match prev {
                Some(v) => std::env::set_var("HOME", v),
                None => std::env::remove_var("HOME"),
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{FsTmp, with_home};

    fn bash(command: &str) -> String {
        serde_json::json!({ "command": command }).to_string()
    }

    /// Register `id` as an exploration with one in-memory chat session
    /// (`session_id`), so promotion has something to migrate.
    fn seed_exploration(state: &mut State, id: &str, session_id: &str) {
        state.change.explorations.push(chat_store::Exploration {
            id: id.to_string(),
            display_name: id.to_string(),
            idea_path: None,
            session_count: 0,
        });
        let mut ix = interaction::InteractionState::default();
        let mut ax = interaction::AgentSession::new(id.to_string(), scope::ScopeKind::Exploration);
        ax.session.id = session_id.to_string();
        ix.sessions.push(ax);
        state
            .interactions
            .insert(scope::Scope::Exploration(id.to_string()), ix);
    }

    /// @spec exploration/promotion Promotion requires an authoritative binding: Bound change adopts its originating exploration
    #[test]
    fn bound_change_adopts_originating_exploration() {
        let tmp = FsTmp::new();
        with_home(tmp.path(), || {
            let mut state = State::new();
            let exp_id = "exploration-1";
            let new_name = "my-change";
            seed_exploration(&mut state, exp_id, "sess-1");
            // GIVEN the exploration's agent created the change → binding recorded.
            state
                .change
                .pending_bindings
                .insert(new_name.to_string(), exp_id.to_string());

            // WHEN promotion is evaluated.
            promote_bound_exploration(&mut state, new_name);

            // THEN the exploration is promoted into the change AND its chat
            // sessions are accessible under the change's scope.
            let change_scope = scope::Scope::Change(new_name.to_string());
            assert!(state.interactions.contains_key(&change_scope));
            assert!(
                !state
                    .interactions
                    .contains_key(&scope::Scope::Exploration(exp_id.to_string()))
            );
            let ix = state.interactions.get(&change_scope).unwrap();
            assert!(ix.sessions.iter().any(|s| s.session.id == "sess-1"));
        });
    }

    /// @spec exploration/promotion Promotion requires an authoritative binding: Unbound change adopts no exploration
    #[test]
    fn unbound_change_adopts_no_exploration() {
        let tmp = FsTmp::new();
        with_home(tmp.path(), || {
            let mut state = State::new();
            let exp_id = "exploration-unrelated";
            seed_exploration(&mut state, exp_id, "sess-x");
            // GIVEN an unrelated exploration is currently selected (UI focus).
            state.active_area = Area::Change;
            state.change.selected_change = Some(exp_id.to_string());
            let new_name = "out-of-band";
            // No binding exists for `new_name`.

            // WHEN promotion is evaluated.
            promote_bound_exploration(&mut state, new_name);

            // THEN no exploration is promoted into the change AND the selected
            // exploration's sessions remain under their own scope.
            assert!(
                !state
                    .interactions
                    .contains_key(&scope::Scope::Change(new_name.to_string()))
            );
            let ix = state
                .interactions
                .get(&scope::Scope::Exploration(exp_id.to_string()))
                .unwrap();
            assert!(ix.sessions.iter().any(|s| s.session.id == "sess-x"));
        });
    }

    // @spec exploration/promotion Chat focus after bound promotion: Bound promotion restores chat input focus
    #[test]
    fn bound_promotion_restores_chat_input_focus() {
        // GIVEN an exploration whose agent created a change by name
        let tmp = FsTmp::new();
        with_home(tmp.path(), || {
            let mut state = State::new();
            let exp_id = "exploration-focus";
            let new_name = "focus-change";
            seed_exploration(&mut state, exp_id, "sess-focus");
            state
                .change
                .pending_bindings
                .insert(new_name.to_string(), exp_id.to_string());

            // WHEN promotion is evaluated
            let promoted = promote_bound_exploration(&mut state, new_name);

            // THEN the exploration is promoted into the change AND chat focus
            // is requested (callers batch focus_chat_input when promoted).
            assert!(
                promoted,
                "bound promotion must request chat input focus"
            );
            assert!(state
                .interactions
                .contains_key(&scope::Scope::Change(new_name.to_string())));
        });
    }

    // @spec exploration/promotion Chat focus after bound promotion: Unbound new change does not force chat input focus
    #[test]
    fn unbound_new_change_does_not_force_chat_input_focus() {
        // GIVEN a newly-present change with no binding AND chat not focused
        let tmp = FsTmp::new();
        with_home(tmp.path(), || {
            let mut state = State::new();
            let exp_id = "exploration-unbound-focus";
            seed_exploration(&mut state, exp_id, "sess-ub");
            state.active_area = Area::Change;
            state.change.selected_change = Some(exp_id.to_string());
            let new_name = "standalone-change";
            // No binding; chat_input_focused stays false (default).

            // WHEN promotion is evaluated
            let promoted = promote_bound_exploration(&mut state, new_name);

            // THEN no exploration is promoted AND focus is not forced
            assert!(
                !promoted,
                "unbound detection must not request chat input focus"
            );
            assert!(!state
                .interactions
                .contains_key(&scope::Scope::Change(new_name.to_string())));
        });
    }

    /// @spec exploration/promotion Bindings are single-use: A consumed binding does not re-promote on reappearance
    #[test]
    fn consumed_binding_does_not_repromote_on_reappearance() {
        let tmp = FsTmp::new();
        with_home(tmp.path(), || {
            let mut state = State::new();
            let exp_id = "exploration-2";
            let new_name = "twice";
            seed_exploration(&mut state, exp_id, "sess-2");
            state
                .change
                .pending_bindings
                .insert(new_name.to_string(), exp_id.to_string());

            // First detection consumes the binding and promotes.
            promote_bound_exploration(&mut state, new_name);
            let change_scope = scope::Scope::Change(new_name.to_string());
            assert!(state.interactions.contains_key(&change_scope));

            // GIVEN the binding is now consumed. A second, unrelated exploration
            // is selected in focus to prove focus still can't re-promote.
            let other = "exploration-3";
            seed_exploration(&mut state, other, "sess-3");
            state.active_area = Area::Change;
            state.change.selected_change = Some(other.to_string());

            // WHEN the same change directory is detected as newly present again.
            promote_bound_exploration(&mut state, new_name);

            // THEN no exploration is promoted into the change again — the other
            // exploration and its session stay under their own scope.
            assert!(
                state
                    .interactions
                    .contains_key(&scope::Scope::Exploration(other.to_string()))
            );
            let ix = state.interactions.get(&change_scope).unwrap();
            assert!(!ix.sessions.iter().any(|s| s.session.id == "sess-3"));
        });
    }

    #[test]
    fn parses_plain_create_change() {
        assert_eq!(
            parse_create_change(&bash("ds create change my-thing")),
            Some("my-thing".to_string())
        );
    }

    #[test]
    fn parses_quoted_multiword_title() {
        assert_eq!(
            parse_create_change(&bash("ds create change \"My Thing\"")),
            Some("my-thing".to_string())
        );
    }

    #[test]
    fn parses_cd_prefixed_and_compound_command() {
        assert_eq!(
            parse_create_change(&bash(
                "cd /repo && ds create change my-thing && ds status"
            )),
            Some("my-thing".to_string())
        );
    }

    /// Grok's shell tool is `run_terminal_command`, not Claude's `Bash` —
    /// attribution keys off the `command` field, not the tool name.
    #[test]
    fn parses_grok_run_terminal_command() {
        assert_eq!(
            parse_create_change(&bash(
                "ds create change md-table-render && ds status"
            )),
            Some("md-table-render".to_string())
        );
    }

    #[test]
    fn ignores_input_without_command_field() {
        // A non-shell tool (Write, read_file, …) has no `command` key.
        let write = r#"{"file_path":"foo.rs","content":"ds create change my-thing"}"#;
        assert_eq!(parse_create_change(write), None);
    }

    #[test]
    fn ignores_command_without_create_change() {
        assert_eq!(parse_create_change(&bash("ds status")), None);
    }
}
