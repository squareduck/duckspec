//! Shared interaction state — terminal + agent chat — used by Change, Caps, and Codex areas.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use iced::Element;

use duckchat::Attachment;

use crate::agent::{AgentHandle, SlashCommand};
use crate::chat_store::ChatSession;
use crate::highlight::SyntaxHighlighter;
use crate::scope::ScopeKind;
use crate::theme;
use crate::widget::{
    agent_chat, collapsible, interaction_toggle, list_view,
    text_edit::{self, Block, EditorState, Pos},
};

// ── Selection context attachments ───────────────────────────────────────────

/// A captured selection from a content tab or chat history block, attached
/// to a chat session so it's included in the next turn(s).
#[derive(Debug, Clone)]
pub struct SelectionContext {
    pub source: SelectionSource,
    pub range: SelectionRange,
    /// Snapshot of the selected text at capture time. Pinned excerpts keep
    /// the original snapshot even if the underlying file or chat block
    /// changes later.
    pub text: String,
}

#[derive(Debug, Clone)]
pub enum SelectionSource {
    /// Selection in a file/diff/idea content tab.
    Tab {
        /// User-facing path or label rendered in the chip and the agent
        /// payload (e.g. `src/main.rs` or `idea: My title`).
        display_path: String,
    },
    /// Selection in a chat history block.
    ChatBlock {
        /// Header label of the block (e.g. `User`, `Assistant`).
        role_label: String,
        /// Position of the block in the rebuilt blocks list at capture time.
        block_idx: usize,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct SelectionRange {
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
}

impl SelectionRange {
    fn from_pos(start: Pos, end: Pos) -> Self {
        Self {
            start_line: start.line,
            start_col: start.col,
            end_line: end.line,
            end_col: end.col,
        }
    }

    /// `L12` for single-line, `L12-24` for multi-line. 1-based.
    pub fn short_label(&self) -> String {
        if self.start_line == self.end_line {
            format!("L{}", self.start_line + 1)
        } else {
            format!("L{}-{}", self.start_line + 1, self.end_line + 1)
        }
    }
}

/// Compute compact labels for a slice of selections. File-sourced labels
/// abbreviate to the filename, with just enough parent path components
/// to keep each label unique within the slice. Chat-block labels are
/// returned unchanged (they're already short).
///
/// Order is preserved: `out[i]` is the label for `items[i]`.
pub fn chip_labels_abbreviated(items: &[&SelectionContext]) -> Vec<String> {
    use std::collections::HashMap;

    // Pre-split path components for tab-sourced items so the inner loop
    // doesn't repeat the work for every k.
    let splits: Vec<Option<Vec<&str>>> = items
        .iter()
        .map(|s| match &s.source {
            SelectionSource::Tab { display_path } => Some(
                display_path
                    .split('/')
                    .filter(|p| !p.is_empty())
                    .collect::<Vec<_>>(),
            ),
            SelectionSource::ChatBlock { .. } => None,
        })
        .collect();

    // Group indices by filename so each group gets its own disambiguation
    // pass. Singleton groups always render as bare filename.
    let mut by_filename: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, parts) in splits.iter().enumerate() {
        if let Some(parts) = parts
            && let Some(name) = parts.last()
        {
            by_filename.entry(*name).or_default().push(i);
        }
    }

    let mut abbrev: HashMap<usize, String> = HashMap::new();
    for indices in by_filename.values() {
        for &i in indices {
            let parts = splits[i].as_ref().expect("tab-sourced index");
            let mut k = 0usize;
            loop {
                let take = (k + 1).min(parts.len());
                let suffix = &parts[parts.len() - take..];
                let unique = indices.iter().all(|&j| {
                    if j == i {
                        return true;
                    }
                    let other = splits[j].as_ref().expect("tab-sourced index");
                    // Pills with identical full paths can't disambiguate
                    // by path — they're the same file with different
                    // ranges. Skip them so the common abbreviation
                    // collapses to the shortest unique form against
                    // *other* paths in the group.
                    if other == parts {
                        return true;
                    }
                    let other_take = take.min(other.len());
                    let other_suffix = &other[other.len() - other_take..];
                    other_suffix != suffix
                });
                if unique || take == parts.len() {
                    abbrev.insert(i, suffix.join("/"));
                    break;
                }
                k += 1;
            }
        }
    }

    items
        .iter()
        .enumerate()
        .map(|(i, sel)| {
            let lines = sel.range.short_label();
            match &sel.source {
                SelectionSource::Tab { display_path } => {
                    let abbr = abbrev
                        .get(&i)
                        .cloned()
                        .unwrap_or_else(|| display_path.clone());
                    format!("{abbr} {lines}")
                }
                SelectionSource::ChatBlock {
                    role_label,
                    block_idx,
                } => format!("chat: {role_label} #{} {lines}", block_idx + 1),
            }
        })
        .collect()
}

/// Render the pinned + tentative selection contexts as a single text blurb
/// suitable for `TurnRequest::system_additions`. Returns `None` for an
/// empty list so the caller can skip pushing.
pub fn render_selection_attachments(items: &[SelectionContext]) -> Option<String> {
    if items.is_empty() {
        return None;
    }
    let mut out = String::from("Selected context attached by the user:\n\n");
    for s in items {
        // Coordinates are emitted as 1-based `line:col` pairs so the agent
        // can map back to the file precisely if it wants to. Lines/cols are
        // both useful: highlighting tools and "show me what comes after"
        // questions both need column-level locations.
        let coords = format!(
            "{}:{}–{}:{}",
            s.range.start_line + 1,
            s.range.start_col + 1,
            s.range.end_line + 1,
            s.range.end_col + 1,
        );
        let header = match &s.source {
            SelectionSource::Tab { display_path } => {
                format!("File: {display_path} ({coords})")
            }
            SelectionSource::ChatBlock {
                role_label,
                block_idx,
            } => format!(
                "Chat excerpt: {role_label} message (block {}, {coords})",
                block_idx + 1
            ),
        };
        out.push_str(&header);
        out.push_str("\n```\n");
        out.push_str(&s.text);
        if !s.text.ends_with('\n') {
            out.push('\n');
        }
        out.push_str("```\n\n");
    }
    Some(out)
}

/// Build a `SelectionContext` from a chat block editor at `idx` in the
/// session, if it has a non-empty selection. The block label drives the
/// chip's role string.
pub fn chat_block_selection(ax: &AgentSession, idx: usize) -> Option<SelectionContext> {
    let editor = ax.chat_editors.get(idx)?;
    let (start, end) = editor.selection_range()?;
    if start == end {
        return None;
    }
    let text = editor.selection_text()?;
    let role_label = ax
        .chat_blocks
        .get(idx)
        .map(|b| b.label.clone())
        .unwrap_or_else(|| "?".to_string());
    Some(SelectionContext {
        source: SelectionSource::ChatBlock {
            role_label,
            block_idx: idx,
        },
        range: SelectionRange::from_pos(start, end),
        text,
    })
}

/// Build a `SelectionContext` from a content tab editor with the given
/// `display_path`, if it has a non-empty selection.
pub fn tab_editor_selection(
    editor: &EditorState,
    display_path: String,
) -> Option<SelectionContext> {
    let (start, end) = editor.selection_range()?;
    if start == end {
        return None;
    }
    let text = editor.selection_text()?;
    Some(SelectionContext {
        source: SelectionSource::Tab { display_path },
        range: SelectionRange::from_pos(start, end),
        text,
    })
}

/// Pin the tentative attachment (if any) into `selection_pinned` and
/// drop the live reference. After this, any visual selection in editors
/// is also cleared so the next selection starts a fresh tentative slot.
pub fn pin_tentative(ax: &mut AgentSession) -> bool {
    let Some(sel) = ax.selection_tentative.take() else {
        return false;
    };
    ax.selection_pinned.push(sel);
    for editor in ax.chat_editors.iter_mut() {
        editor.anchor = None;
    }
    true
}

/// Clear all attachments on the session and any in-chat selection state
/// so the user starts clean.
pub fn clear_all_attachments(ax: &mut AgentSession) {
    ax.selection_pinned.clear();
    ax.selection_tentative = None;
    for editor in ax.chat_editors.iter_mut() {
        editor.anchor = None;
    }
}

/// Monotonic counter used to mint a stable `InteractionState::instance_id`.
/// The ID keys long-lived subscriptions (PTY, agent) so they survive when the
/// interaction's scope name changes (e.g. exploration promoted to a real change).
static NEXT_INSTANCE_ID: AtomicU64 = AtomicU64::new(1);

// ── Active interaction tab ──────────────────────────────────────────────────

/// Which tab is currently selected in the interaction column.
/// Chat is implicit (always present); terminals are stored as a `Vec`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    Chat,
    Terminal(usize),
}

impl Default for ActiveTab {
    fn default() -> Self {
        ActiveTab::Chat
    }
}

/// One terminal tab — owns its TerminalState and a stable id used as the
/// PTY subscription key. Display label is derived from position in
/// `InteractionState::terminals` (`Term {idx + 1}`).
pub struct TerminalTab {
    pub id: u64,
    pub state: crate::widget::terminal::TerminalState,
}

// ── Session controls (which buttons to show) ────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionControls {
    /// Show a session dropdown + "+" new-session button.
    Multi,
    /// Show a "Clear" button that resets the single session.
    Single,
}

// ── Agent session (per-session bundle) ──────────────────────────────────────

pub struct AgentSession {
    pub session: ChatSession,
    /// What kind of duckspec object this session belongs to. Runtime-only
    /// (not persisted) — set by the caller when sessions are created or
    /// loaded. Drives the `CurrentScopeHook` blurb on the first turn.
    pub scope_kind: ScopeKind,
    pub agent_handle: Option<AgentHandle>,
    pub chat_input: EditorState,
    /// Transient per-input attachment side table: id → bytes/media_type/label.
    /// Populated by `AttachImage` paste actions, drained into the
    /// `TurnRequest` on send.
    pub input_attachments: HashMap<String, Attachment>,
    pub chat_commands: Vec<SlashCommand>,
    pub chat_completion: agent_chat::CompletionState,
    pub chat_blocks: Vec<Block>,
    pub chat_editors: Vec<EditorState>,
    pub chat_collapsed: Vec<bool>,
    pub esc_count: u8,
    pub agent_model: String,
    pub agent_input_tokens: usize,
    pub agent_output_tokens: usize,
    pub agent_context_window: usize,
    /// Suggested /ds-* command for the current stage (without the leading slash).
    /// Used as the "press Enter on empty input" shortcut and for placeholder text.
    pub obvious_command: Option<String>,
    /// Pending message staged while the agent is streaming. Sent automatically
    /// when the current turn ends (either naturally or via user-triggered
    /// interrupt). `None` means the queue is empty.
    pub queue_editor: Option<EditorState>,
    /// Latest known kanban-card description for this session's scope, if
    /// any. Populated by the kanban area whenever a card is opened or its
    /// description is edited. Not persisted — rehydrated from disk on next
    /// open. `send_prompt_text` compares this against
    /// `session.last_seeded_description` to decide whether to inject the
    /// description as system context on the upcoming turn.
    pub card_description: Option<String>,
    /// True when the chat transcript is at (or near) the bottom — driven by
    /// the scrollable's `on_scroll` callback. Streaming events use this to
    /// decide whether to auto-scroll the view: stays put while the user is
    /// reading history, snaps to bottom when they're already there.
    pub stick_to_bottom: bool,
    /// Last `absolute_offset.y` we saw from the chat scrollable. Used by the
    /// `ChatScrolled` handler to tell user-driven scroll-ups (offset
    /// decreased) apart from content-grew-under-viewport notifications
    /// (offset unchanged but content bounds grew). Without this distinction
    /// the latter would race the auto-snap task and unstick us.
    pub last_chat_offset_y: Option<f32>,
    /// Set by `send_prompt_text` when the user submits while sticking to the
    /// bottom: the user's message lands in the transcript immediately but the
    /// agent's first event may take a moment, so the auto-snap path keyed on
    /// `AgentEvent` can't help. Drained by `main` after the dispatch returns
    /// to issue a one-shot `snap_to_end` task.
    pub pending_snap_to_bottom: bool,
    /// Selection-context attachments kept across messages. Built by Cmd-K
    /// (pin tentative) and cleared by Cmd-R (reset).
    pub selection_pinned: Vec<SelectionContext>,
    /// The "live" attachment that mirrors the user's current selection in
    /// the active content tab or a chat history block. Included on the next
    /// turn alongside `selection_pinned` and dropped after send (Cmd-K is
    /// the explicit gesture to keep it).
    pub selection_tentative: Option<SelectionContext>,
    /// Heuristic flag tracking whether the chat input is the focus target.
    /// Set true on `InputAction`, false when focus moves to a chat block or
    /// a content tab editor. Used to gate Cmd-R so it only fires when the
    /// user is plausibly typing into the chat.
    pub chat_input_focused: bool,
}

impl AgentSession {
    /// Create a fresh session for a scope.
    pub fn new(scope: String, scope_kind: ScopeKind) -> Self {
        Self::from_session(ChatSession::new(scope), scope_kind)
    }

    /// Wrap a loaded ChatSession with fresh UI state.
    pub fn from_session(session: ChatSession, scope_kind: ScopeKind) -> Self {
        Self {
            session,
            scope_kind,
            agent_handle: None,
            chat_input: EditorState::new(""),
            input_attachments: HashMap::new(),
            chat_commands: Vec::new(),
            chat_completion: agent_chat::CompletionState::default(),
            chat_blocks: Vec::new(),
            chat_editors: Vec::new(),
            chat_collapsed: Vec::new(),
            esc_count: 0,
            agent_model: String::new(),
            agent_input_tokens: 0,
            agent_output_tokens: 0,
            agent_context_window: 200_000,
            obvious_command: None,
            queue_editor: None,
            card_description: None,
            stick_to_bottom: true,
            last_chat_offset_y: None,
            pending_snap_to_bottom: false,
            selection_pinned: Vec::new(),
            selection_tentative: None,
            chat_input_focused: false,
        }
    }
}

// ── Interaction state ───────────────────────────────────────────────────────

pub struct InteractionState {
    /// Stable ID for subscription routing. Set once at construction and never
    /// changed — in particular, promoting an exploration to a real change moves
    /// the `InteractionState` between HashMap keys but leaves this untouched,
    /// so the underlying PTY / agent subscriptions survive the rename.
    pub instance_id: u64,
    pub visible: bool,
    pub width: f32,
    /// Currently selected tab.
    pub active_tab: ActiveTab,
    /// Terminal tabs (chat is implicit at the start of the bar).
    pub terminals: Vec<TerminalTab>,
    /// Monotonic id for the next terminal tab in this scope. Used as the
    /// stable PTY subscription key so output keeps routing to the right tab
    /// even after reorders/removals.
    pub next_terminal_id: u64,
    /// True when the active tab is a terminal *and* it should capture
    /// keyboard input. Cleared by overlays (file finder) to release focus
    /// without closing the panel.
    pub terminal_focused: bool,
    // Agent sessions (sorted newest-first).
    pub sessions: Vec<AgentSession>,
    pub active_session: usize,
    /// Whether the multi-session "CHATS" section is expanded.
    pub chat_section_expanded: bool,
}

impl Default for InteractionState {
    fn default() -> Self {
        Self {
            instance_id: NEXT_INSTANCE_ID.fetch_add(1, Ordering::Relaxed),
            visible: false,
            width: theme::INTERACTION_COLUMN_WIDTH,
            active_tab: ActiveTab::Chat,
            terminals: Vec::new(),
            next_terminal_id: 1,
            terminal_focused: false,
            sessions: Vec::new(),
            active_session: 0,
            chat_section_expanded: false,
        }
    }
}

impl InteractionState {
    pub fn active(&self) -> Option<&AgentSession> {
        self.sessions.get(self.active_session)
    }

    pub fn active_mut(&mut self) -> Option<&mut AgentSession> {
        self.sessions.get_mut(self.active_session)
    }

    pub fn find_session_mut(&mut self, id: &str) -> Option<&mut AgentSession> {
        self.sessions.iter_mut().find(|s| s.session.id == id)
    }

    pub fn find_session_index(&self, id: &str) -> Option<usize> {
        self.sessions.iter().position(|s| s.session.id == id)
    }

    /// The terminal tab currently shown, if any.
    pub fn active_terminal(&self) -> Option<&TerminalTab> {
        match self.active_tab {
            ActiveTab::Terminal(i) => self.terminals.get(i),
            ActiveTab::Chat => None,
        }
    }

    pub fn active_terminal_mut(&mut self) -> Option<&mut TerminalTab> {
        match self.active_tab {
            ActiveTab::Terminal(i) => self.terminals.get_mut(i),
            ActiveTab::Chat => None,
        }
    }

    pub fn find_terminal_index(&self, id: u64) -> Option<usize> {
        self.terminals.iter().position(|t| t.id == id)
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_skips_empty_list() {
        assert!(render_selection_attachments(&[]).is_none());
    }

    #[test]
    fn render_includes_path_coords_and_text() {
        let sel = SelectionContext {
            source: SelectionSource::Tab {
                display_path: "src/main.rs".into(),
            },
            range: SelectionRange {
                start_line: 11,
                start_col: 4,
                end_line: 23,
                end_col: 0,
            },
            text: "let x = 1;\n".into(),
        };
        let out = render_selection_attachments(&[sel]).unwrap();
        assert!(out.contains("File: src/main.rs (12:5–24:1)"), "header: {out}");
        assert!(out.contains("```\nlet x = 1;\n```"), "body: {out}");
    }

    #[test]
    fn render_chat_excerpt_uses_role_and_block() {
        let sel = SelectionContext {
            source: SelectionSource::ChatBlock {
                role_label: "User".into(),
                block_idx: 2,
            },
            range: SelectionRange {
                start_line: 0,
                start_col: 0,
                end_line: 0,
                end_col: 5,
            },
            text: "hello".into(),
        };
        let out = render_selection_attachments(&[sel]).unwrap();
        assert!(out.contains("Chat excerpt: User message (block 3"), "header: {out}");
        assert!(out.contains("```\nhello\n```"), "body: {out}");
    }

    fn tab_sel(path: &str, start: usize, end: usize) -> SelectionContext {
        SelectionContext {
            source: SelectionSource::Tab {
                display_path: path.into(),
            },
            range: SelectionRange {
                start_line: start,
                start_col: 0,
                end_line: end,
                end_col: 0,
            },
            text: String::new(),
        }
    }

    #[test]
    fn chip_label_collapses_single_line_range() {
        let sel = tab_sel("dir/a.rs", 4, 4);
        let labels = chip_labels_abbreviated(&[&sel]);
        assert_eq!(labels, vec!["a.rs L5".to_string()]);
    }

    #[test]
    fn chip_label_uses_range_for_multi_line() {
        let sel = tab_sel("dir/a.rs", 4, 9);
        let labels = chip_labels_abbreviated(&[&sel]);
        assert_eq!(labels, vec!["a.rs L5-10".to_string()]);
    }

    #[test]
    fn chip_labels_abbreviate_single_pill_to_filename() {
        let sel = tab_sel("crates/duckboard/src/main.rs", 0, 2);
        let labels = chip_labels_abbreviated(&[&sel]);
        assert_eq!(labels, vec!["main.rs L1-3".to_string()]);
    }

    #[test]
    fn chip_labels_disambiguate_with_one_parent() {
        let a = tab_sel("crates/foo/spec.delta.md", 0, 0);
        let b = tab_sel("crates/bar/spec.delta.md", 0, 0);
        let labels = chip_labels_abbreviated(&[&a, &b]);
        assert_eq!(
            labels,
            vec![
                "foo/spec.delta.md L1".to_string(),
                "bar/spec.delta.md L1".to_string(),
            ]
        );
    }

    #[test]
    fn chip_labels_collide_at_same_parent_walk_higher() {
        let a = tab_sel("x/y/a/foo.md", 0, 0);
        let b = tab_sel("x/y/b/foo.md", 0, 0);
        let c = tab_sel("x/z/a/foo.md", 0, 0);
        let labels = chip_labels_abbreviated(&[&a, &b, &c]);
        // a vs c both share `a/foo.md`; need one more parent (`y/a/foo.md`
        // vs `z/a/foo.md`). b's `b/foo.md` is already unique.
        assert_eq!(
            labels,
            vec![
                "y/a/foo.md L1".to_string(),
                "b/foo.md L1".to_string(),
                "z/a/foo.md L1".to_string(),
            ]
        );
    }

    #[test]
    fn chip_labels_same_path_different_ranges_collapse_to_filename() {
        // Two selections in the same file should both render as the
        // shortest unique form against *other* files, not fight each
        // other for disambiguation.
        let a = tab_sel("a/b/c/file.md", 0, 0);
        let b = tab_sel("a/b/c/file.md", 4, 6);
        let labels = chip_labels_abbreviated(&[&a, &b]);
        assert_eq!(
            labels,
            vec!["file.md L1".to_string(), "file.md L5-7".to_string()]
        );
    }

    #[test]
    fn chip_labels_one_unique_plus_two_duplicates_abbreviate_correctly() {
        // Mirrors the user's bug report: one pill at `nonexistent/...`
        // plus two identical pills at `auth/...`. Both auth pills should
        // collapse to one parent (`auth/`), not the full path.
        let a = tab_sel("crates/x/y/nonexistent/spec.delta.md", 4, 4);
        let b = tab_sel("crates/x/y/auth/spec.delta.md", 6, 6);
        let c = tab_sel("crates/x/y/auth/spec.delta.md", 10, 14);
        let labels = chip_labels_abbreviated(&[&a, &b, &c]);
        assert_eq!(
            labels,
            vec![
                "nonexistent/spec.delta.md L5".to_string(),
                "auth/spec.delta.md L7".to_string(),
                "auth/spec.delta.md L11-15".to_string(),
            ]
        );
    }

    #[test]
    fn chip_labels_mixed_sources_keep_chat_label_intact() {
        let file = tab_sel("dir/a.rs", 0, 0);
        let chat = SelectionContext {
            source: SelectionSource::ChatBlock {
                role_label: "User".into(),
                block_idx: 4,
            },
            range: SelectionRange {
                start_line: 0,
                start_col: 0,
                end_line: 1,
                end_col: 0,
            },
            text: String::new(),
        };
        let labels = chip_labels_abbreviated(&[&file, &chat]);
        assert_eq!(
            labels,
            vec!["a.rs L1".to_string(), "chat: User #5 L1-2".to_string()]
        );
    }
}

// ── Shared messages ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Msg {
    Handle(interaction_toggle::HandleMsg),
    /// Switch which interaction tab (chat or a specific terminal) is shown.
    SelectTab(ActiveTab),
    /// Spawn a new terminal tab and select it.
    AddTerminal,
    /// Close the terminal tab at the given index.
    CloseTerminal(usize),
    AgentChat(agent_chat::Msg),
    TerminalScroll,
    /// User cmd-clicked a hyperlink in the terminal output. Handled by main.
    TerminalOpenUrl(String),
    /// Create a new agent session for the current scope. Handled by area.
    NewSession,
    /// Switch the active agent session by id. Handled by area.
    SelectSession(String),
    /// Reset the active session (single-session UIs). Handled by area.
    ClearSession,
    /// Collapse / expand the multi-session list.
    ToggleChatSection,
}

// ── Update helpers ──────────────────────────────────────────────────────────

/// Handle an interaction message. Returns `true` if the panel was just toggled open.
/// NewSession / SelectSession / ClearSession are ignored here — areas handle them.
pub fn update(state: &mut InteractionState, msg: Msg, highlighter: &SyntaxHighlighter) -> bool {
    let mut just_opened = false;
    match msg {
        Msg::Handle(hmsg) => match hmsg {
            interaction_toggle::HandleMsg::Toggle => {
                state.visible = !state.visible;
                just_opened = state.visible;
            }
            interaction_toggle::HandleMsg::SetWidth(w) => {
                state.width = w;
            }
        },
        Msg::SelectTab(tab) => {
            // Clamp Terminal(i) to a valid index; if invalid, fall back to Chat.
            state.active_tab = match tab {
                ActiveTab::Terminal(i) if i < state.terminals.len() => ActiveTab::Terminal(i),
                ActiveTab::Terminal(_) => ActiveTab::Chat,
                ActiveTab::Chat => ActiveTab::Chat,
            };
        }
        Msg::AddTerminal => {
            if let Some(idx) = spawn_new_terminal(state) {
                state.active_tab = ActiveTab::Terminal(idx);
            }
        }
        Msg::CloseTerminal(idx) => {
            if idx < state.terminals.len() {
                state.terminals.remove(idx);
                state.active_tab = adjust_active_after_remove(state.active_tab, idx);
            }
        }
        Msg::AgentChat(chat_msg) => {
            handle_agent_chat(state, chat_msg, highlighter);
        }
        Msg::TerminalScroll => {
            if let Some(tt) = state.active_terminal_mut() {
                tt.state.apply_scroll();
            }
        }
        Msg::NewSession | Msg::SelectSession(_) | Msg::ClearSession => {
            // Area-handled.
        }
        Msg::TerminalOpenUrl(url) => {
            if let Err(err) = opener::open(&url) {
                tracing::warn!(%url, %err, "failed to open terminal URL");
            }
        }
        Msg::ToggleChatSection => {
            state.chat_section_expanded = !state.chat_section_expanded;
        }
    }
    just_opened
}

fn handle_agent_chat(
    state: &mut InteractionState,
    msg: agent_chat::Msg,
    highlighter: &SyntaxHighlighter,
) {
    let Some(ax) = state.active_mut() else { return };
    match msg {
        agent_chat::Msg::InputAction(action) => {
            // Any keyboard/mouse activity in the chat input implies focus is
            // on it — flip the heuristic so Cmd-R is allowed to clear.
            ax.chat_input_focused = true;
            if let text_edit::EditorAction::OpenUrl(url) = &action {
                if let Err(err) = opener::open(url) {
                    tracing::warn!(%url, %err, "failed to open chat URL");
                }
                return;
            }
            if let text_edit::EditorAction::AttachImage { id, label, media_type, bytes } = action {
                let link = format!("[{label}](attach:{id})");
                ax.input_attachments.insert(
                    id,
                    Attachment {
                        label,
                        media_type,
                        bytes,
                    },
                );
                ax.chat_input.apply_action(text_edit::EditorAction::Paste(link));
                rehighlight_input(&mut ax.chat_input, highlighter);
                return;
            }
            if ax.chat_completion.visible {
                match &action {
                    text_edit::EditorAction::MoveUp(_) => {
                        completion_prev(ax);
                        return;
                    }
                    text_edit::EditorAction::MoveDown(_) => {
                        completion_next(ax);
                        return;
                    }
                    _ => {}
                }
            }
            // Backspace on an empty input discards a queued message instead of
            // bouncing off the start of the editor.
            if matches!(action, text_edit::EditorAction::Backspace)
                && ax.queue_editor.is_some()
                && ax.chat_input.text().is_empty()
            {
                ax.queue_editor = None;
                return;
            }
            let mutated = ax.chat_input.apply_action(action);
            if mutated {
                rehighlight_input(&mut ax.chat_input, highlighter);
            }
            let input_text = ax.chat_input.text();
            let trimmed = input_text.trim_end();
            if trimmed.starts_with('/') && !trimmed.contains(' ') {
                ax.chat_completion.visible = true;
                ax.chat_completion.selected = 0;
            } else {
                ax.chat_completion.visible = false;
            }
        }
        agent_chat::Msg::CompletionNext => completion_next(ax),
        agent_chat::Msg::CompletionPrev => completion_prev(ax),
        agent_chat::Msg::CompletionAccept => completion_accept(ax, highlighter),
        agent_chat::Msg::CompletionDismiss => {
            ax.chat_completion.visible = false;
        }
        agent_chat::Msg::ChatAction(idx, action) => {
            // Focus moved off the chat input. Also enforce single-source
            // selection: clear anchors in OTHER chat editors so the most
            // recent gesture wins the tentative slot.
            ax.chat_input_focused = false;
            for (i, editor) in ax.chat_editors.iter_mut().enumerate() {
                if i != idx {
                    editor.anchor = None;
                }
            }
            // Whether the tentative attachment should be recomputed after
            // this action. Skip for in-flight drags: the chip appearing
            // mid-drag would reflow the chat panel under the user's
            // cursor and the drag would target the wrong content. Click
            // and DragEnd refresh; everything else (keyboard nav,
            // copy, …) refreshes too.
            let refresh = !matches!(&action, text_edit::EditorAction::Drag(_));
            if let Some(editor) = ax.chat_editors.get_mut(idx) {
                handle_chat_action_on(editor, action);
            }
            if refresh {
                refresh_tentative_from_chat(ax, idx);
            }
        }
        agent_chat::Msg::ToggleCollapse(idx) => {
            if let Some(collapsed) = ax.chat_collapsed.get_mut(idx) {
                *collapsed = !*collapsed;
            }
        }
        agent_chat::Msg::SendPressed => {
            let typed = ax.chat_input.text().trim().to_string();

            if ax.session.is_streaming {
                if !typed.is_empty() {
                    // Streaming + text in input → stage/append to queue,
                    // clear input. Never interrupts.
                    let combined = match ax.queue_editor.as_ref() {
                        Some(q) => format!("{}\n\n{}", q.text(), typed),
                        None => typed,
                    };
                    ax.queue_editor = Some(make_queue_editor(&combined, highlighter));
                    ax.chat_input = EditorState::new("");
                    rehighlight_input(&mut ax.chat_input, highlighter);
                    ax.chat_completion.visible = false;
                } else if ax.queue_editor.is_some() {
                    // Streaming + empty input + queue present → interrupt.
                    // The queue will auto-flush when TurnComplete arrives.
                    if let Some(handle) = &ax.agent_handle {
                        handle.cancel();
                    }
                }
                // Streaming + empty input + no queue → no-op.
            } else {
                let typed_opt = if typed.is_empty() {
                    ax.obvious_command.as_ref().map(|c| format!("/{c}"))
                } else {
                    Some(typed)
                };
                let queued = ax.queue_editor.take().map(|q| q.text());
                let text = match (queued, typed_opt) {
                    (Some(q), Some(t)) => Some(format!("{q}\n\n{t}")),
                    (Some(q), None) => Some(q),
                    (None, Some(t)) => Some(t),
                    (None, None) => None,
                };
                if let Some(text) = text {
                    send_prompt_text(ax, text, highlighter);
                }
            }
        }
        agent_chat::Msg::CancelPressed => {
            if let Some(handle) = &ax.agent_handle {
                handle.cancel();
            }
        }
        agent_chat::Msg::QueueAction(action) => {
            if let text_edit::EditorAction::OpenUrl(url) = &action {
                if let Err(err) = opener::open(url) {
                    tracing::warn!(%url, %err, "failed to open chat URL");
                }
                return;
            }
            if !action.is_mutating()
                && let Some(ed) = ax.queue_editor.as_mut()
            {
                ed.apply_action(action);
            }
        }
        agent_chat::Msg::DiscardQueue => {
            ax.queue_editor = None;
        }
        agent_chat::Msg::ChatScrolled(viewport) => {
            let bounds = viewport.bounds();
            let content = viewport.content_bounds();
            let offset_y = viewport.absolute_offset().y;
            let max_scroll = (content.height - bounds.height).max(0.0);
            let distance_from_bottom = (max_scroll - offset_y).max(0.0);
            let at_bottom =
                distance_from_bottom <= agent_chat::STICK_TO_BOTTOM_THRESHOLD;

            // The scrollable publishes viewport notifications for both
            // user-driven scrolls *and* content-size changes (via
            // `RedrawRequested`). To avoid racing the auto-snap task, only
            // disengage stick on a clear user scroll-up (offset decreased);
            // only engage when actually within threshold of the bottom.
            // Same-offset events caused by content growing underneath are
            // preserved.
            let prev_offset = ax.last_chat_offset_y;
            ax.last_chat_offset_y = Some(offset_y);
            if at_bottom {
                ax.stick_to_bottom = true;
            } else if let Some(prev) = prev_offset
                && offset_y + f32::EPSILON < prev
            {
                ax.stick_to_bottom = false;
            }
        }
    }
}

/// Recompute `selection_tentative` from a chat block at `idx`. If the
/// block has a non-empty selection, the tentative becomes a `ChatBlock`
/// source pointing at it (overriding any prior tentative). If selection
/// was just cleared in this block (or it had none) and the existing
/// tentative was a chat-sourced one — including from a *different* block,
/// since `ChatAction` clears those — drop the tentative. File-sourced
/// tentatives are left untouched.
pub fn refresh_tentative_from_chat(ax: &mut AgentSession, idx: usize) {
    if let Some(sel) = chat_block_selection(ax, idx) {
        ax.selection_tentative = Some(sel);
        return;
    }
    if matches!(
        ax.selection_tentative.as_ref().map(|s| &s.source),
        Some(SelectionSource::ChatBlock { .. })
    ) {
        ax.selection_tentative = None;
    }
}

/// Set the tentative attachment from a content tab editor selection. Any
/// chat-sourced tentative is dropped; chat block anchors are cleared so
/// the user's most recent gesture wins the tentative slot.
///
/// `display_path` and `tab_id` are caller-supplied so this module doesn't
/// have to know about `tab_bar::TabView` shapes — see main.rs where
/// content-tab editor actions are handled.
pub fn set_tentative_from_tab(
    ax: &mut AgentSession,
    editor: &EditorState,
    display_path: String,
) {
    if let Some(sel) = tab_editor_selection(editor, display_path) {
        for chat_editor in ax.chat_editors.iter_mut() {
            chat_editor.anchor = None;
        }
        ax.selection_tentative = Some(sel);
    } else if matches!(
        ax.selection_tentative.as_ref().map(|s| &s.source),
        Some(SelectionSource::Tab { .. })
    ) {
        ax.selection_tentative = None;
    }
}

/// Build a read-only queue editor with markdown highlighting applied so the
/// queue pill reads like a regular chat message.
fn make_queue_editor(text: &str, highlighter: &SyntaxHighlighter) -> EditorState {
    let mut editor = EditorState::new(text);
    let syntax = highlighter.find_syntax("md");
    editor.highlight_spans = Some(highlighter.highlight_lines(&editor.lines, syntax));
    editor
}

/// Send `text` as a new user turn on the active agent handle. Pushes the user
/// message into the session, marks streaming, clears the input, and rebuilds
/// the chat editor blocks. No-op if no agent handle is attached.
pub fn send_prompt_text(ax: &mut AgentSession, text: String, highlighter: &SyntaxHighlighter) {
    use duckchat::{ContextHook, TurnRequest};

    let Some(handle) = &ax.agent_handle else {
        return;
    };
    // Fallback: if we have prior messages but no Claude session to `--resume`,
    // prepend the history as context so the agent isn't starting blind.
    // Happens for legacy sessions saved before session-id persistence, or if
    // the server-side session has been pruned.
    let prompt = if ax.session.claude_session_id.is_none() && !ax.session.messages.is_empty() {
        build_history_preamble(&ax.session.messages) + &text
    } else {
        text.clone()
    };

    // First time Claude sees this conversation — include a scope orientation
    // blurb so the agent doesn't have to ask which change/exploration/etc.
    // we're in. Subsequent turns ride `--resume` and skip this.
    let mut system_additions = Vec::new();
    if ax.session.claude_session_id.is_none() {
        let scope = crate::scope::SessionScope {
            kind: ax.scope_kind,
            scope_key: ax.session.scope.clone(),
        };
        if let Some(out) = crate::scope::CurrentScopeHook.compute(&scope) {
            system_additions.push(out.text);
        }
    }

    // Selection-context attachments: pinned + tentative, in that order,
    // prepended to the per-turn prompt. They can't ride
    // `system_additions` — that maps to `--append-system-prompt` on the
    // claude CLI and only takes effect on the first invocation; later
    // turns reuse the resumed session's baked-in system prompt and would
    // silently drop the attachments.
    let prompt = {
        let mut attached: Vec<SelectionContext> = ax.selection_pinned.clone();
        if let Some(t) = ax.selection_tentative.as_ref() {
            attached.push(t.clone());
        }
        match render_selection_attachments(&attached) {
            Some(blurb) => {
                tracing::info!(
                    pinned = ax.selection_pinned.len(),
                    tentative = ax.selection_tentative.is_some(),
                    blurb_chars = blurb.chars().count(),
                    "prepending selection-context blurb to prompt"
                );
                format!("{blurb}{prompt}")
            }
            None => prompt,
        }
    };

    // Card description: inject on the first turn (when non-empty), and
    // re-inject on any later turn where the description has changed since
    // we last told the agent. Empty descriptions are skipped — if the card
    // later gains a description, the diff against stored `None` will trigger
    // an inject at that point.
    if let Some(desc) = ax.card_description.as_ref()
        && !desc.trim().is_empty()
        && ax.session.last_seeded_description.as_deref() != Some(desc.as_str())
    {
        let blurb = if ax.session.last_seeded_description.is_none() {
            format!("Card description:\n\n{desc}")
        } else {
            format!("Card description (updated since last turn):\n\n{desc}")
        };
        system_additions.push(blurb);
        ax.session.last_seeded_description = Some(desc.clone());
    }

    ax.session.messages.push(crate::chat_store::ChatMessage {
        role: crate::chat_store::Role::User,
        content: vec![crate::chat_store::ContentBlock::Text(text)],
        timestamp: String::new(),
    });
    ax.session.is_streaming = true;
    ax.session.pending_text.clear();
    // The user's message just grew the transcript. If they were stuck to the
    // bottom we want them to see it land there immediately — without this
    // flag the next auto-snap waits for the first `AgentEvent`.
    if ax.stick_to_bottom {
        ax.pending_snap_to_bottom = true;
    }

    let mut req = TurnRequest::new(prompt, handle.working_dir().to_path_buf());
    req.system_additions = system_additions;
    req.attachments = std::mem::take(&mut ax.input_attachments);
    handle.send_turn(req);

    ax.chat_input = EditorState::new("");
    rehighlight_input(&mut ax.chat_input, highlighter);
    ax.chat_completion.visible = false;
    // Drop the tentative attachment — it rode this turn but is not pinned.
    // Pinned attachments persist across messages until Cmd-R clears them.
    ax.selection_tentative = None;
    rebuild_chat_editor(ax, highlighter);
}

/// Re-run markdown syntax highlighting on the chat input.
fn rehighlight_input(input: &mut EditorState, highlighter: &SyntaxHighlighter) {
    let syntax = highlighter.find_syntax("md");
    input.highlight_spans = Some(highlighter.highlight_lines(&input.lines, syntax));
}

/// Render prior chat history as a text preamble for the agent. Used when we
/// don't have a Claude `--resume` session id but need to hand the agent
/// context from earlier turns. Returns a block ending with a separator; the
/// caller appends the new user message after it.
fn build_history_preamble(messages: &[crate::chat_store::ChatMessage]) -> String {
    use crate::chat_store::{ContentBlock, Role};

    let mut out = String::from("Previous conversation in this chat (for context):\n\n");
    for msg in messages {
        let who = match msg.role {
            Role::User => "User",
            Role::Assistant => "Assistant",
            Role::System => "System",
        };
        for block in &msg.content {
            match block {
                ContentBlock::Text(t) => {
                    out.push_str(who);
                    out.push_str(": ");
                    out.push_str(t);
                    out.push_str("\n\n");
                }
                ContentBlock::ToolUse { name, .. } => {
                    out.push_str(&format!("[{who} invoked tool: {name}]\n\n"));
                }
                ContentBlock::ToolResult { name, .. } => {
                    out.push_str(&format!("[tool result: {name}]\n\n"));
                }
            }
        }
    }
    out.push_str("---\n\nContinue the conversation. New user message:\n\n");
    out
}

// ── Chat editor ─────────────────────────────────────────────────────────────

/// Rebuild the per-block chat editors for the given session.
pub fn rebuild_chat_editor(ax: &mut AgentSession, highlighter: &SyntaxHighlighter) {
    let new_blocks = agent_chat::build_chat_blocks(&ax.session);

    let old_len = ax.chat_collapsed.len();
    ax.chat_collapsed.resize(new_blocks.len(), false);
    for (i, block) in new_blocks.iter().enumerate().skip(old_len) {
        // Collapse tool blocks on first appearance regardless of current
        // content — during streaming they show up empty and get filled in
        // later, and we don't want them flashing expanded then snapping shut.
        ax.chat_collapsed[i] = matches!(
            block.kind,
            crate::widget::text_edit::BlockKind::ToolUse
                | crate::widget::text_edit::BlockKind::ToolResult
        );
    }

    let mut new_editors = Vec::with_capacity(new_blocks.len());
    for (i, block) in new_blocks.iter().enumerate() {
        if i < ax.chat_editors.len()
            && i < ax.chat_blocks.len()
            && ax.chat_blocks[i].lines == block.lines
        {
            let existing = std::mem::replace(&mut ax.chat_editors[i], EditorState::new(""));
            new_editors.push(existing);
        } else {
            let content = block.lines.join("\n");
            let mut editor = EditorState::new(&content);
            let syntax = highlighter.find_syntax("md");
            editor.highlight_spans = Some(highlighter.highlight_lines(&editor.lines, syntax));
            new_editors.push(editor);
        }
    }

    ax.chat_editors = new_editors;
    ax.chat_blocks = new_blocks;
}

fn handle_chat_action_on(editor: &mut EditorState, action: crate::widget::text_edit::EditorAction) {
    if let crate::widget::text_edit::EditorAction::OpenUrl(url) = &action {
        if let Err(err) = opener::open(url) {
            tracing::warn!(%url, %err, "failed to open chat URL");
        }
        return;
    }
    // Chat editors are read-only — skip mutating actions.
    if !action.is_mutating() {
        editor.apply_action(action);
    }
}

// ── Agent chat keyboard routing ────────────────────────────────────────────

/// Result of handling an agent-chat keyboard event.
pub enum AgentChatKeyResult {
    /// The key was consumed; caller should return `Task::none()`.
    Handled,
    /// The key maps to a chat message to dispatch through the update cycle.
    Dispatch(agent_chat::Msg),
    /// The key was not consumed by agent chat keyboard handling.
    NotHandled,
}

/// Handle agent-chat-specific keyboard shortcuts: completion navigation,
/// Esc-Esc cancel, Enter to send, Shift+Enter for newline. Returns how the
/// caller should proceed.
pub fn handle_agent_chat_key(
    ix: &mut InteractionState,
    key: &iced::keyboard::Key,
    mods: iced::keyboard::Modifiers,
) -> AgentChatKeyResult {
    use iced::keyboard;
    use iced::keyboard::key::Named;

    let Some(ax) = ix.active_mut() else {
        return AgentChatKeyResult::NotHandled;
    };

    // Completion shortcuts (Tab, Esc, Ctrl+N/P) when popup is visible.
    if ax.chat_completion.visible {
        let completion_msg = match key {
            keyboard::Key::Named(Named::Tab) => Some(agent_chat::Msg::CompletionAccept),
            keyboard::Key::Named(Named::Escape) => Some(agent_chat::Msg::CompletionDismiss),
            _ if mods.control() && *key == keyboard::Key::Character("n".into()) => {
                Some(agent_chat::Msg::CompletionNext)
            }
            _ if mods.control() && *key == keyboard::Key::Character("p".into()) => {
                Some(agent_chat::Msg::CompletionPrev)
            }
            _ => None,
        };
        if let Some(msg) = completion_msg {
            return AgentChatKeyResult::Dispatch(msg);
        }
    }

    // Esc-Esc to cancel streaming.
    if *key == keyboard::Key::Named(Named::Escape) && ax.session.is_streaming {
        ax.esc_count += 1;
        if ax.esc_count >= 2 {
            return AgentChatKeyResult::Dispatch(agent_chat::Msg::CancelPressed);
        }
        return AgentChatKeyResult::Handled;
    }

    // Reset esc counter on any non-Esc key.
    if *key != keyboard::Key::Named(Named::Escape) {
        ax.esc_count = 0;
    }

    // Enter-to-send is handled by the chat input's TextEdit widget via
    // `on_submit`, so it only fires when the input is focused. Shift+Enter
    // falls through to the default Enter action which inserts a newline.

    AgentChatKeyResult::NotHandled
}

// ── Completion helpers ──────────────────────────────────────────────────────

fn completion_next(ax: &mut AgentSession) {
    let input_text = ax.chat_input.text();
    let query = input_text.trim_end().trim_start_matches('/');
    let count = agent_chat::filter_commands(&ax.chat_commands, query).len();
    if count > 0 {
        ax.chat_completion.selected = (ax.chat_completion.selected + 1) % count;
    }
}

fn completion_prev(ax: &mut AgentSession) {
    let input_text = ax.chat_input.text();
    let query = input_text.trim_end().trim_start_matches('/');
    let count = agent_chat::filter_commands(&ax.chat_commands, query).len();
    if count > 0 {
        ax.chat_completion.selected = if ax.chat_completion.selected == 0 {
            count - 1
        } else {
            ax.chat_completion.selected - 1
        };
    }
}

fn completion_accept(ax: &mut AgentSession, highlighter: &SyntaxHighlighter) {
    let input_text = ax.chat_input.text();
    let query = input_text.trim_end().trim_start_matches('/');
    let filtered = agent_chat::filter_commands(&ax.chat_commands, query);
    let selected = ax
        .chat_completion
        .selected
        .min(filtered.len().saturating_sub(1));
    if let Some(&(cmd_idx, _)) = filtered.get(selected) {
        let cmd_name = &ax.chat_commands[cmd_idx].name;
        let new_text = format!("/{} ", cmd_name);
        let mut new_state = EditorState::new(&new_text);
        let last_line = new_state.lines.len().saturating_sub(1);
        let last_col = new_state.lines[last_line].len();
        new_state.cursor = text_edit::Pos::new(last_line, last_col);
        ax.chat_input = new_state;
        rehighlight_input(&mut ax.chat_input, highlighter);
    }
    ax.chat_completion.visible = false;
}

// ── High-level update with side effects ────────────────────────────────────

/// Handle an interaction message with the standard side effects: ensure
/// agent sessions exist while the chat tab is showing, and keep the
/// `terminal_focused` latch in sync with the active tab + visibility.
/// Suitable for the common `other =>` arm shared by Caps, Codex, and Change.
pub fn update_with_side_effects(
    state: &mut InteractionState,
    msg: Msg,
    scope: &str,
    scope_label: &str,
    scope_kind: ScopeKind,
    project_root: Option<&std::path::Path>,
    highlighter: &SyntaxHighlighter,
) {
    update(state, msg, highlighter);

    if state.visible && state.active_tab == ActiveTab::Chat {
        ensure_sessions_with_label(
            state,
            scope,
            scope_label,
            scope_kind,
            project_root,
            highlighter,
        );
    }

    state.terminal_focused =
        state.visible && matches!(state.active_tab, ActiveTab::Terminal(_));
}

// ── Session management ─────────────────────────────────────────────────────

/// Clear and reset the active session for single-session areas (Caps, Codex,
/// pre-promotion ideas).
///
/// `scope` is the on-disk key; `scope_label` is the human-readable label used
/// when the session has no `title`. They differ for ideas (label = idea title,
/// scope = `exploration-…` id); caps/codex pass the same string for both.
pub fn clear_single_session(
    ix: &mut InteractionState,
    scope: &str,
    scope_label: &str,
    scope_kind: ScopeKind,
    project_root: Option<&std::path::Path>,
) {
    if ix.sessions.is_empty() {
        let mut ax = AgentSession::new(scope.to_string(), scope_kind);
        reconcile_display_names(std::slice::from_mut(&mut ax), scope_label);
        ix.sessions.push(ax);
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
    reconcile_display_names(&mut ix.sessions, scope_label);
}

// ── Spawn helpers ───────────────────────────────────────────────────────────

/// Spawn a fresh terminal tab and append it to `state.terminals`. Returns the
/// new tab's index, or `None` if the PTY/emulator failed to construct (the
/// error is logged).
pub fn spawn_new_terminal(state: &mut InteractionState) -> Option<usize> {
    match crate::widget::terminal::TerminalState::new() {
        Ok(ts) => {
            let id = state.next_terminal_id;
            state.next_terminal_id += 1;
            let idx = state.terminals.len();
            state.terminals.push(TerminalTab { id, state: ts });
            tracing::info!(id, "terminal spawned");
            Some(idx)
        }
        Err(e) => {
            tracing::error!("failed to create terminal: {e}");
            None
        }
    }
}

/// Recompute `active_tab` after removing the terminal at `removed_idx`.
///   - If the active tab was the one removed, fall back to the previous
///     terminal (or Chat if it was the first).
///   - If the active tab sat to the right of the removed one, shift its
///     index down by one to keep pointing at the same logical tab.
pub fn adjust_active_after_remove(active: ActiveTab, removed_idx: usize) -> ActiveTab {
    match active {
        ActiveTab::Chat => ActiveTab::Chat,
        ActiveTab::Terminal(active_idx) if active_idx == removed_idx => {
            if removed_idx == 0 {
                ActiveTab::Chat
            } else {
                ActiveTab::Terminal(removed_idx - 1)
            }
        }
        ActiveTab::Terminal(active_idx) if active_idx > removed_idx => {
            ActiveTab::Terminal(active_idx - 1)
        }
        ActiveTab::Terminal(active_idx) => ActiveTab::Terminal(active_idx),
    }
}

/// Ensure the interaction has at least one session for the scope.
///
/// On first call, loads any persisted sessions; if none, creates one empty.
/// `scope` is the on-disk key (directory name); `scope_label` is the
/// human-readable label shown in the session dropdown (may differ — e.g. an
/// exploration's display_name vs. its stable id).
pub fn ensure_sessions_with_label(
    state: &mut InteractionState,
    scope: &str,
    scope_label: &str,
    scope_kind: ScopeKind,
    project_root: Option<&std::path::Path>,
    highlighter: &SyntaxHighlighter,
) {
    if !state.sessions.is_empty() {
        return;
    }
    let loaded = crate::chat_store::load_sessions_for(scope, project_root);
    if loaded.is_empty() {
        let mut ax = AgentSession::new(scope.to_string(), scope_kind);
        reconcile_display_names(std::slice::from_mut(&mut ax), scope_label);
        state.sessions.push(ax);
    } else {
        for session in loaded {
            let mut ax = AgentSession::from_session(session, scope_kind);
            rebuild_chat_editor(&mut ax, highlighter);
            state.sessions.push(ax);
        }
        // Re-reconcile with the caller's preferred label (load_sessions_for
        // used the raw scope key, which is wrong for explorations).
        reconcile_display_names(&mut state.sessions, scope_label);
    }
    state.active_session = 0;
}

/// Re-run display-name reconciliation on a slice of `AgentSession`.
/// Call after inserting a new session or promoting scopes.
///
/// `scope_label` is the human-readable scope label (change name, exploration
/// display_name, etc.); sessions with a `title` override it.
pub fn reconcile_display_names(sessions: &mut [AgentSession], scope_label: &str) {
    use std::collections::HashMap;
    let label_for = |ax: &AgentSession| -> String {
        ax.session
            .title
            .clone()
            .unwrap_or_else(|| scope_label.to_string())
    };
    let mut groups: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, ax) in sessions.iter().enumerate() {
        let prefix = crate::chat_store::minute_prefix_public(ax.session.created_at_nanos);
        groups.entry(prefix).or_default().push(i);
    }
    for (_prefix, mut indices) in groups {
        indices.sort_by_key(|&i| sessions[i].session.created_at_nanos);
        if indices.len() == 1 {
            let i = indices[0];
            let minute =
                crate::chat_store::minute_prefix_public(sessions[i].session.created_at_nanos);
            let label = label_for(&sessions[i]);
            sessions[i].session.display_name = format!("{minute} {label}");
        } else {
            for (n, i) in indices.iter().enumerate() {
                let minute =
                    crate::chat_store::minute_prefix_public(sessions[*i].session.created_at_nanos);
                let label = label_for(&sessions[*i]);
                sessions[*i].session.display_name = format!("{minute} #{} {label}", n + 1);
            }
        }
    }
}

// ── Shared area layout ────────────────────────────────────────────────────

// ── View ────────────────────────────────────────────────────────────────────

/// View the interaction column content (mode tabs + session controls + terminal/agent chat).
pub fn view_column<'a, M: 'a + Clone>(
    state: &'a InteractionState,
    wrap: impl Fn(Msg) -> M + 'a + Clone,
    controls: SessionControls,
) -> Element<'a, M> {
    use iced::widget::column;

    let mode_tabs = view_interaction_tabs(state, wrap.clone());

    let content: Element<'a, M> = match state.active_tab {
        ActiveTab::Terminal(i) => {
            if let Some(tt) = state.terminals.get(i) {
                let w = wrap.clone();
                crate::widget::terminal::view_terminal(&tt.state).map(move |ev| match ev {
                    crate::widget::terminal::TerminalEvent::Redraw => w(Msg::TerminalScroll),
                    crate::widget::terminal::TerminalEvent::OpenUrl(url) => {
                        w(Msg::TerminalOpenUrl(url))
                    }
                })
            } else {
                view_placeholder(wrap.clone())
            }
        }
        ActiveTab::Chat => {
            if let Some(ax) = state.active() {
                let status = agent_chat::StatusInfo {
                    is_streaming: ax.session.is_streaming,
                    esc_count: ax.esc_count,
                    model: if ax.agent_model.is_empty() {
                        "\u{2014}".to_string()
                    } else {
                        ax.agent_model.clone()
                    },
                    context_tokens: ax.agent_input_tokens + ax.agent_output_tokens,
                    context_max: ax.agent_context_window,
                };
                let w = wrap.clone();
                let chat_view = agent_chat::view(
                    &ax.session,
                    &ax.chat_blocks,
                    &ax.chat_editors,
                    &ax.chat_collapsed,
                    &ax.chat_input,
                    ax.queue_editor.as_ref(),
                    &ax.chat_commands,
                    &ax.chat_completion,
                    status,
                    ax.obvious_command.as_deref(),
                    &ax.selection_pinned,
                    ax.selection_tentative.as_ref(),
                )
                .map(move |m| w(Msg::AgentChat(m)));

                let session_bar = view_session_bar(state, controls, wrap.clone());
                column![session_bar, chat_view]
                    .height(iced::Length::Fill)
                    .into()
            } else {
                view_placeholder(wrap.clone())
            }
        }
    };

    column![mode_tabs, content]
        .height(iced::Length::Fill)
        .into()
}

fn view_session_bar<'a, M: 'a + Clone>(
    state: &'a InteractionState,
    controls: SessionControls,
    wrap: impl Fn(Msg) -> M + 'a + Clone,
) -> Element<'a, M> {
    use iced::Length;
    use iced::widget::{Space, button, column, container, row, text};

    let bar_border = container(Space::new().width(Length::Fill).height(1.0))
        .width(Length::Fill)
        .style(theme::divider);

    match controls {
        SessionControls::Single => {
            let w = wrap.clone();
            let clear_btn = button(text("Clear").size(theme::font_sm()))
                .on_press(w(Msg::ClearSession))
                .padding([2.0, theme::SPACING_SM])
                .style(theme::session_bar_button);

            // Layout budget: SM padding on each side of the bar, the Clear
            // button (its own SM padding around the label), an XS spacer
            // between label and button, and a 4px safety margin so the title
            // text doesn't kiss the button's edge.
            let clear_w = measure_text("Clear", theme::font_sm()) + theme::SPACING_SM * 2.0;
            let overhead = theme::SPACING_SM * 2.0 + theme::SPACING_XS + clear_w + 4.0;
            let available = (state.width - overhead).max(0.0);
            let active_name = state.active().map(|ax| ax.session.display_name.as_str());
            let label = active_name
                .map(|n| truncate_to_width(n, available, theme::font_sm()))
                .unwrap_or_default();

            let row = row![
                text(label)
                    .size(theme::font_sm())
                    .color(theme::text_secondary())
                    .wrapping(iced::widget::text::Wrapping::None),
                Space::new().width(Length::Fill),
                clear_btn,
            ]
            .spacing(theme::SPACING_XS)
            .align_y(iced::Center);

            column![
                container(row)
                    .padding([theme::SPACING_XS, theme::SPACING_SM])
                    .width(Length::Fill)
                    .style(theme::surface),
                bar_border,
            ]
            .into()
        }
        SessionControls::Multi => {
            let expanded = state.chat_section_expanded;

            let active_name = state.active().map(|ax| ax.session.display_name.as_str());
            // Header layout: chevron + spacing + label inside a button with
            // SM horizontal padding on each side, then a sibling `+` button
            // (its own SM padding around the icon). 4px safety margin so the
            // last glyph doesn't kiss the plus button's edge.
            let chevron_w = theme::font_sm();
            let plus_w = theme::font_sm() + theme::SPACING_SM * 2.0;
            let overhead = theme::SPACING_SM * 2.0 + chevron_w + theme::SPACING_XS + plus_w + 4.0;
            let available = (state.width - overhead).max(0.0);
            let label_text = match active_name {
                Some(name) => truncate_to_width(name, available, theme::font_sm()),
                None => "CHATS".to_string(),
            };

            let w_toggle = wrap.clone();
            let header_btn = button(
                row![
                    collapsible::chevron(expanded),
                    text(label_text)
                        .size(theme::font_sm())
                        .color(theme::text_secondary())
                        .wrapping(iced::widget::text::Wrapping::None),
                ]
                .spacing(theme::SPACING_XS)
                .align_y(iced::Center)
                .width(Length::Fill),
            )
            .on_press(w_toggle(Msg::ToggleChatSection))
            .width(Length::Fill)
            .style(theme::section_header)
            .padding([theme::SPACING_XS, theme::SPACING_SM]);

            let w_new = wrap.clone();
            let plus_btn = collapsible::add_button(w_new(Msg::NewSession));

            let header_row = row![container(header_btn).width(Length::Fill), plus_btn,];

            let mut section = column![header_row].spacing(0.0);

            if expanded {
                section = section.push(collapsible::top_divider());
                let active_id = state.active().map(|a| a.session.id.as_str());
                let mut rows: Vec<list_view::ListRow<'a, M>> = Vec::new();
                for s in &state.sessions {
                    let is_selected = active_id == Some(s.session.id.as_str());
                    let w_sel = wrap.clone();
                    rows.push(
                        list_view::ListRow::new(s.session.display_name.as_str())
                            .selected(is_selected)
                            .on_press(w_sel(Msg::SelectSession(s.session.id.clone()))),
                    );
                }
                section = section.push(list_view::view(rows, None));
            }

            column![section, bar_border].spacing(0.0).into()
        }
    }
}

/// Measure the rendered width of `text` at `size` using iced's default UI font
/// (matches what `text(...)` renders without a `.font()` override).
pub(crate) fn measure_text(text: &str, size: f32) -> f32 {
    measure_text_with_shaping(text, size, iced::widget::text::Shaping::Basic)
}

/// Measure with `Shaping::Advanced` — the shaper `text_input` uses internally.
/// Use this when sizing a container around a text_input so the field's
/// width matches what the widget will actually render at.
pub(crate) fn measure_text_advanced(text: &str, size: f32) -> f32 {
    measure_text_with_shaping(text, size, iced::widget::text::Shaping::Advanced)
}

fn measure_text_with_shaping(
    text: &str,
    size: f32,
    shaping: iced::widget::text::Shaping,
) -> f32 {
    use iced::advanced::graphics::text::Paragraph;
    use iced::advanced::text::Paragraph as _;
    let t = iced::advanced::text::Text {
        content: text,
        bounds: iced::Size::INFINITE,
        size: iced::Pixels(size),
        line_height: iced::widget::text::LineHeight::default(),
        font: iced::Font::DEFAULT,
        align_x: iced::advanced::text::Alignment::Left,
        align_y: iced::alignment::Vertical::Top,
        shaping,
        wrapping: iced::widget::text::Wrapping::None,
    };
    Paragraph::with_text(t).min_bounds().width
}

/// Truncate `name` (with a trailing `…`) so that the rendered width fits in
/// `available_px`. Returns the original `name` if it already fits, or just
/// `…` if no characters fit.
fn truncate_to_width(name: &str, available_px: f32, font_size: f32) -> String {
    const ELLIPSIS: &str = "\u{2026}";
    if available_px <= 0.0 {
        return ELLIPSIS.to_string();
    }
    if measure_text(name, font_size) <= available_px {
        return name.to_string();
    }
    let chars: Vec<char> = name.chars().collect();
    // Binary search for the longest prefix whose `prefix + …` still fits.
    let (mut lo, mut hi) = (0usize, chars.len());
    while lo < hi {
        let mid = lo + (hi - lo).div_ceil(2);
        let candidate: String = chars[..mid].iter().collect::<String>() + ELLIPSIS;
        if measure_text(&candidate, font_size) <= available_px {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    chars[..lo].iter().collect::<String>() + ELLIPSIS
}

/// Tab bar for the interaction column: pinned `Chat` tab, then a closable
/// `Term {n}` tab per terminal, then a trailing `+ Terminal` button. The
/// tab row scrolls horizontally if it overflows. Mirrors the styling used
/// by the content column's tab bar (`widget::tab_bar::view_bar`).
fn view_interaction_tabs<'a, M: 'a + Clone>(
    state: &'a InteractionState,
    wrap: impl Fn(Msg) -> M + 'a + Clone,
) -> Element<'a, M> {
    use iced::Length;
    use iced::widget::{Space, button, column, container, row, scrollable, svg, text};

    type TabStyle = fn(&iced::Theme, iced::widget::button::Status) -> iced::widget::button::Style;

    let separator = || -> Element<'a, M> {
        let h = theme::font_sm() * 1.3 + 2.0 * theme::SPACING_XS;
        container(Space::new().width(1.0).height(h))
            .style(theme::divider)
            .into()
    };

    let style_for = |is_active: bool| -> TabStyle {
        if is_active {
            theme::tab_active as TabStyle
        } else {
            theme::tab_inactive as TabStyle
        }
    };

    let mut tabs_row = row![].spacing(0.0);

    // Pinned chat tab.
    let chat_active = state.active_tab == ActiveTab::Chat;
    let w_chat = wrap.clone();
    let chat_btn = button(text("Chat").size(theme::font_sm()))
        .on_press(w_chat(Msg::SelectTab(ActiveTab::Chat)))
        .padding([theme::SPACING_XS, theme::SPACING_MD])
        .style(style_for(chat_active));
    tabs_row = tabs_row.push(chat_btn);

    // One closable tab per terminal. Label is derived from index.
    for (i, _tt) in state.terminals.iter().enumerate() {
        tabs_row = tabs_row.push(separator());

        let is_active = state.active_tab == ActiveTab::Terminal(i);
        let label = format!("Term {}", i + 1);

        let w_close = wrap.clone();
        let close_btn = collapsible::close_button(w_close(Msg::CloseTerminal(i)));

        let tab_row = row![
            text(label).size(theme::font_sm()),
            Space::new().width(theme::SPACING_SM),
            close_btn,
        ]
        .spacing(theme::SPACING_XS)
        .align_y(iced::Center);

        // Asymmetric padding so the × hugs the right edge — matches the
        // content column's closable tabs.
        let pad = iced::Padding {
            top: theme::SPACING_XS,
            right: theme::SPACING_SM,
            bottom: theme::SPACING_XS,
            left: theme::SPACING_MD,
        };

        let w_sel = wrap.clone();
        let tab_btn = button(tab_row)
            .on_press(w_sel(Msg::SelectTab(ActiveTab::Terminal(i))))
            .padding(pad)
            .style(style_for(is_active));
        tabs_row = tabs_row.push(tab_btn);
    }

    // Separator + "+ Terminal" add button + trailing cap separator.
    tabs_row = tabs_row.push(separator());

    let plus_icon = svg(svg::Handle::from_memory(collapsible::ICON_PLUS))
        .width(theme::font_sm())
        .height(theme::font_sm())
        .style(theme::svg_tint(theme::text_secondary()));
    let add_label = row![
        plus_icon,
        text("Terminal")
            .size(theme::font_sm())
            .color(theme::text_secondary()),
    ]
    .spacing(theme::SPACING_XS)
    .align_y(iced::Center);
    let w_add = wrap.clone();
    let add_btn = button(add_label)
        .on_press(w_add(Msg::AddTerminal))
        .padding([theme::SPACING_XS, theme::SPACING_MD])
        .style(theme::tab_inactive as TabStyle);
    tabs_row = tabs_row.push(add_btn);
    tabs_row = tabs_row.push(separator());

    let tabs_scroll = scrollable(tabs_row)
        .direction(theme::thin_scrollbar_direction_horizontal())
        .style(theme::thin_scrollbar)
        .width(Length::Fill);

    let bar_border = container(Space::new().width(Length::Fill).height(1.0))
        .width(Length::Fill)
        .style(theme::divider);

    column![
        container(tabs_scroll)
            .width(Length::Fill)
            .style(theme::tab_bar),
        bar_border,
    ]
    .into()
}

fn view_placeholder<'a, M: 'a>(_wrap: impl Fn(Msg) -> M + 'a) -> Element<'a, M> {
    use iced::widget::{Space, column, container, text};

    container(
        column![
            text("Interaction")
                .size(theme::font_md())
                .color(theme::text_secondary()),
            Space::new().height(theme::SPACING_MD),
            text("Terminal and chat will appear here.")
                .size(theme::font_md())
                .color(theme::text_muted()),
        ]
        .spacing(theme::SPACING_SM)
        .padding(theme::SPACING_LG),
    )
    .width(iced::Length::Fill)
    .height(iced::Length::Fill)
    .into()
}
