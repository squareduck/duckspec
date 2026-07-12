//! Shared interaction state — terminal + agent chat — used by Change, Caps, and Codex areas.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use iced::Element;

use duckchat::{Attachment, ModelRef};

use std::path::Path;

use crate::agent::{AgentHandle, SlashCommand};
use crate::chat_store::{ChatMessage, ChatSession, ContentBlock, Role};
use crate::highlight::SyntaxHighlighter;
use crate::scope::ScopeKind;
use crate::theme;
use crate::widget::{
    agent_chat, collapsible, interaction_toggle, list_view,
    text_edit::{self, Block, EditorState, Pos},
};

/// Appended to the system prompt on a session's first turn so the model's
/// file references match what `path_link` detects — project-relative paths
/// with an optional 1-based `:line` suffix become cmd-clickable in the
/// chat panel.
const PATH_REFERENCE_NOTE: &str = "When referencing project files in your replies, \
    write the path relative to the project root, optionally with a 1-based line \
    suffix — e.g. `crates/duckboard/src/main.rs:42`. Prefer this form over absolute \
    paths or bare filenames; the UI turns such references into clickable links.";

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActiveTab {
    #[default]
    Chat,
    Terminal(usize),
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
    /// Segment-index-aligned collapse state (user override + auto-collapse).
    pub chat_collapse: Vec<agent_chat::CollapseState>,
    pub esc_count: u8,
    /// Resolved project-level default model (harness-tagged `ModelRef`) for
    /// this session's project. Transient (not persisted) — refreshed from
    /// `Config` by the main loop. Used to render the picker's "Default (…)"
    /// label and to resolve the effective model when `session.selected_model`
    /// is `None`.
    pub project_model_default: Option<ModelRef>,
    /// Set when the user changes the per-chat model via the picker; consumed
    /// by `update_with_side_effects` to persist the session. Transient.
    pub model_dirty: bool,
    pub agent_input_tokens: usize,
    pub agent_output_tokens: usize,
    /// Multi-option fast-response shell (empty until a live user choice fills
    /// it). Ephemeral — not persisted.
    pub fast_response: crate::fast_response::FastResponse,
    /// True while a mid-turn structured choice is pending. Chips stay visible
    /// even though `is_streaming` remains true for the open turn.
    pub is_awaiting_user: bool,
    /// Settled oneshot reply strings (0–3); may fill fast-response chips when eligible.
    /// Ephemeral — not persisted. Never seeded into `next_actions`.
    pub agent_default_prompts: Vec<String>,
    /// Monotonic generation; oneshot results apply only when gen matches.
    pub default_prompts_gen: u64,
    /// True while a reply-suggestion oneshot is outstanding for
    /// `default_prompts_gen`. Does not disarm next-action empty Enter / Tab.
    pub default_prompts_pending: bool,
    /// Empty-composer next actions (inherited, lifecycle bootstrap, or trailing
    /// `next`). Ephemeral — not persisted. Refreshed from last assistant /
    /// scope facts / inherited list.
    pub next_actions: Vec<crate::meta_card::NextAction>,
    /// Active index into `next_actions` for ghost / empty Enter / Tab cycle.
    pub next_action_idx: usize,
    /// Donor list for empty-session ghost continuity after NewSession.
    /// Ephemeral — not persisted. Cleared when the session is no longer empty.
    pub inherited_next_actions: Option<Vec<crate::meta_card::NextAction>>,
    /// Lifecycle facts for this session's change scope (phase, step progress,
    /// next stage). Refreshed alongside `fast_response`; `None` for
    /// non-change scopes. Feeds the first-turn scope orientation blurb.
    pub scope_facts: Option<crate::area::change::ChangeScopeFacts>,
    /// Pending message staged while the agent is streaming. Sent automatically
    /// when the current turn ends (either naturally or via user-triggered
    /// interrupt). `None` means the queue is empty.
    pub queue_editor: Option<EditorState>,
    /// Latest known idea body for this session's scope, if any. Populated by
    /// the ideas area whenever an idea is opened or its body is edited. Not
    /// persisted — rehydrated from disk on next open. `send_prompt_text`
    /// compares this against `session.last_seeded_description` to decide
    /// whether to inject the body as system context on the upcoming turn.
    pub idea_description: Option<String>,
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
    /// Spacer above fast response so short history pins chips above the
    /// composer inside the scroll column. Recomputed from scroll/measure
    /// bounds via [`crate::fast_response::bottom_pad`]. Ephemeral.
    pub fast_response_top_pad: f32,
    /// Set by `send_prompt_text` when the user submits while sticking to the
    /// bottom: the user's message lands in the transcript immediately but the
    /// agent's first event may take a moment, so the auto-snap path keyed on
    /// `AgentEvent` can't help. Drained by `main` after the dispatch returns
    /// to issue a one-shot `snap_to_end` task.
    pub pending_snap_to_bottom: bool,
    /// Accumulated edge auto-scroll delta (logical px) from a chat message
    /// whose drag ran past the chat fold. Drained by `main` into a
    /// `scroll_to` on the outer chat scrollable. `None` when idle.
    pub pending_chat_autoscroll: Option<f32>,
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
    /// True while a synthetic AGENTS.md priming turn is in flight on a
    /// fresh session. Set when `send_prompt_text` chooses the two-turn
    /// path; cleared in the `TurnComplete` handler so the user's actual
    /// follow-up message can be dispatched and the title summariser can
    /// fire on the right turn.
    pub priming_in_flight: bool,
    /// User's intended first message, stashed when priming is dispatched
    /// in its place. Drained on the priming turn's `TurnComplete` and
    /// re-fed through `send_prompt_text` against the now-resumable session.
    /// Cleared on cancel/error so a backed-out priming doesn't strand a
    /// phantom command.
    pub pending_followup_prompt: Option<String>,
    /// True between a user cancel and that turn's `TurnComplete`. Deltas can
    /// keep arriving until the agent actually stops; the `TurnComplete`
    /// handler re-captures the unsynced draft when this is set so late text
    /// the user saw is part of the resync. Reset when a new turn starts.
    pub cancel_in_flight: bool,
    /// True when messages have been streamed into this session since its last
    /// persist. Set by the message-mutating `AgentEvent`s, cleared by the
    /// coalesced eager flush and the turn-boundary save. Transient — never
    /// persisted.
    pub needs_flush: bool,
    /// True when `session` transcript may have changed since the last
    /// `materialize_chat_ui`. Pure content/reasoning deltas set this without
    /// rebuilding editors; `StreamTick` and structural events drain it.
    /// Transient — never persisted.
    pub chat_ui_dirty: bool,
}

impl AgentSession {
    /// Create a fresh session for a scope.
    pub fn new(scope: String, scope_kind: ScopeKind) -> Self {
        Self::from_session(ChatSession::new(scope), scope_kind)
    }

    /// Wrap a loaded ChatSession with fresh UI state.
    pub fn from_session(session: ChatSession, scope_kind: ScopeKind) -> Self {
        // Seed the live meter from last-known usage so restart shows fill
        // without waiting for a new UsageUpdate.
        let context_tokens = session.context_tokens;
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
            chat_collapse: Vec::new(),
            esc_count: 0,
            project_model_default: None,
            model_dirty: false,
            agent_input_tokens: context_tokens,
            agent_output_tokens: 0,
            fast_response: crate::fast_response::FastResponse::default(),
            is_awaiting_user: false,
            agent_default_prompts: Vec::new(),
            default_prompts_gen: 0,
            default_prompts_pending: false,
            next_actions: Vec::new(),
            next_action_idx: 0,
            inherited_next_actions: None,
            scope_facts: None,
            queue_editor: None,
            idea_description: None,
            stick_to_bottom: true,
            last_chat_offset_y: None,
            fast_response_top_pad: 0.0,
            pending_snap_to_bottom: false,
            pending_chat_autoscroll: None,
            selection_pinned: Vec::new(),
            selection_tentative: None,
            chat_input_focused: false,
            priming_in_flight: false,
            pending_followup_prompt: None,
            cancel_in_flight: false,
            needs_flush: false,
            chat_ui_dirty: false,
        }
    }

    /// Invalidate in-flight reply-suggestion oneshots and drop agent defaults.
    /// No oneshot is outstanding for the new gen, so readiness is ready with an
    /// empty list until the next TurnComplete spawns one. Called when a turn
    /// starts streaming. Does not clear `next_actions`. Clears oneshot-hint
    /// chips when not awaiting a user choice.
    pub fn clear_agent_default_prompts(&mut self) {
        self.default_prompts_gen = self.default_prompts_gen.wrapping_add(1);
        self.agent_default_prompts.clear();
        self.default_prompts_pending = false;
        if !self.is_awaiting_user {
            self.fast_response = crate::fast_response::clear();
            self.fast_response_top_pad = 0.0;
        }
    }

    /// Mark a new reply-suggestion oneshot as in flight for the current gen.
    pub fn begin_default_prompts_oneshot(&mut self) {
        self.default_prompts_gen = self.default_prompts_gen.wrapping_add(1);
        self.agent_default_prompts.clear();
        self.default_prompts_pending = true;
    }

    /// First lifecycle option for empty-session next-action bootstrap.
    /// Exploration → explore; change → facts ladder head; caps/codex → none.
    /// Independent of agent input hints (oneshot-only gate).
    pub fn lifecycle_bootstrap(&self) -> Option<&str> {
        match self.scope_kind {
            ScopeKind::Exploration => Some("ds-explore"),
            ScopeKind::Change => self
                .scope_facts
                .as_ref()
                .and_then(|f| f.next_command.as_deref()),
            ScopeKind::Caps | ScopeKind::Codex => None,
        }
    }

    /// Rebuild `next_actions` from empty-session inherited list or lifecycle
    /// bootstrap, or the trailing `next` card on the last non-priming
    /// assistant message.
    ///
    /// `after_turn`: pass true from TurnComplete so the ghost starts at the
    /// first ranked action. Chrome/scope rebuilds pass false so Tab cycle is
    /// preserved while the list is unchanged.
    pub fn refresh_next_actions(&mut self, after_turn: bool) {
        let session_empty = self.session.messages.is_empty();
        if !session_empty {
            self.inherited_next_actions = None;
        }
        let bootstrap = self.lifecycle_bootstrap();
        let last_assistant = if session_empty {
            None
        } else {
            crate::default_prompts::last_assistant_and_user(&self.session)
                .map(|(a, _)| a)
        };
        let prev_sends: Vec<String> =
            self.next_actions.iter().map(|a| a.send.clone()).collect();
        let prev_idx = self.next_action_idx;
        // Clone so list rebuild can take owned slices without fighting other
        // borrows of `self` (bootstrap points into scope_facts).
        let inherited = self.inherited_next_actions.clone();
        self.next_actions = crate::default_prompts::next_action_list(
            session_empty,
            bootstrap,
            last_assistant.as_deref(),
            inherited.as_deref(),
        );
        let prev_refs: Vec<&str> = prev_sends.iter().map(String::as_str).collect();
        let new_refs: Vec<&str> = self.next_actions.iter().map(|a| a.send.as_str()).collect();
        self.next_action_idx = crate::default_prompts::next_action_idx_after_refresh(
            after_turn,
            &prev_refs,
            &new_refs,
            prev_idx,
        );
    }

    /// Settled oneshot display list when agent input hints is enabled.
    /// Never includes next-card actions or lifecycle bootstrap.
    pub fn session_oneshot_prompts(&self, agent_input_hints: bool) -> Vec<String> {
        crate::default_prompts::oneshot_display_prompts(
            &self.agent_default_prompts,
            agent_input_hints,
        )
    }

    /// The harness this session's next turn dispatches to, resolved through the
    /// model cascade (per-chat pin → project default → built-in default).
    pub(crate) fn effective_harness(&self) -> String {
        resolve_turn_model(
            self.session.selected_model.as_ref(),
            self.project_model_default.as_ref(),
        )
        .harness
    }

    /// The stored agent session id, but only when it belongs to the harness the
    /// next turn will run on. Session ids are harness-specific — a Claude id
    /// can't `session/load` under grok and vice versa — so when the chat has
    /// been switched to a different harness this returns `None` and the turn
    /// starts a fresh agent-side session (the transcript is re-fed as a history
    /// preamble). Sessions saved before harnesses existed carry no owner and are
    /// treated as `claude-code`.
    pub(crate) fn resumable_session_id(&self) -> Option<&str> {
        let id = self.session.agent_session_id.as_deref()?;
        let owner = self
            .session
            .session_harness
            .as_deref()
            .unwrap_or("claude-code");
        (owner == self.effective_harness()).then_some(id)
    }
}

// ── Content / chat free-space geometry ──────────────────────────────────────

/// Free width for the content ↔ interaction split: window minus fixed left
/// chrome (sidebar, list, their 1px dividers) and the interaction handle.
/// Matches `view_area_three_column` + outer sidebar row geometry.
pub fn free_content_chat_width(window_w: f32) -> f32 {
    let fixed = theme::SIDEBAR_WIDTH
        + 1.0 // sidebar_divider
        + theme::LIST_COLUMN_WIDTH
        + 1.0 // list divider
        + interaction_toggle::HANDLE_WIDTH;
    (window_w - fixed).max(0.0)
}

/// Uncustomized interaction column width: half of free space, floored at min.
pub fn equal_interaction_width(window_w: f32) -> f32 {
    (free_content_chat_width(window_w) / 2.0).max(interaction_toggle::MIN_PANEL_WIDTH)
}

/// Recompute equal width when the panel has not been grip-customized.
pub fn rebalance_uncustomized(ix: &mut InteractionState, window_w: f32) {
    if !ix.width_customized {
        ix.width = equal_interaction_width(window_w);
    }
}

/// Force-show the interaction panel and rebalance uncustomized width from the
/// live window. Same equal-half rule as door open; does not mark customized.
pub fn show_panel(ix: &mut InteractionState, window_w: f32) {
    ix.visible = true;
    rebalance_uncustomized(ix, window_w);
}

/// How the interaction column is sized in the three-column row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InteractionColumnSize {
    /// Content is shown — fixed absolute/equal width.
    Fixed(f32),
    /// Content is hidden — fill remaining free space after left chrome.
    Fill,
}

/// Resolve interaction column size from whether content is shown and the
/// remembered split width (used only when content is visible).
pub fn interaction_column_size(show_content: bool, width: f32) -> InteractionColumnSize {
    if show_content {
        InteractionColumnSize::Fixed(width)
    } else {
        InteractionColumnSize::Fill
    }
}

/// Whether the content column is shown in a three-column area.
/// True only when there is at least one open tab and content is not collapsed.
pub fn show_content_column(has_tabs: bool, content_collapsed: bool) -> bool {
    has_tabs && !content_collapsed
}

// ── Interaction state ───────────────────────────────────────────────────────

pub struct InteractionState {
    /// Stable ID for subscription routing. Set once at construction and never
    /// changed — in particular, promoting an exploration to a real change moves
    /// the `InteractionState` between HashMap keys but leaves this untouched,
    /// so the underlying PTY / agent subscriptions survive the rename.
    pub instance_id: u64,
    pub visible: bool,
    /// True when the content column is collapsed and this panel fills its
    /// space (the door dragged fully open). `width` still holds the remembered
    /// split width to restore to.
    pub content_collapsed: bool,
    pub width: f32,
    /// False until the user first middle-grip sets width. Session memory only.
    pub width_customized: bool,
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

impl InteractionState {
    /// Same as [`Default`], but equal width from free space for `window_w`.
    pub fn for_window(window_w: f32) -> Self {
        Self {
            instance_id: NEXT_INSTANCE_ID.fetch_add(1, Ordering::Relaxed),
            visible: false,
            content_collapsed: false,
            width: equal_interaction_width(window_w),
            width_customized: false,
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

impl Default for InteractionState {
    fn default() -> Self {
        Self::for_window(theme::DEFAULT_WINDOW_WIDTH)
    }
}

/// Build a fresh empty session, optionally inheriting next actions from the
/// current active session (change multi-session only).
///
/// Call while `ix.active()` is still the donor — before inserting the new
/// session at index 0. When `scope_kind` is Change and the donor has a
/// non-empty `next_actions` list, that list is sticky-inherited; otherwise
/// the empty session follows normal lifecycle bootstrap once scope facts are
/// present. Active index starts at 0.
pub fn new_session_with_inherited_next_actions(
    ix: &InteractionState,
    scope_key: String,
    scope_kind: ScopeKind,
) -> AgentSession {
    let donor = ix.active();
    let donor_actions = match (scope_kind, donor) {
        (ScopeKind::Change, Some(d)) if !d.next_actions.is_empty() => {
            Some(d.next_actions.clone())
        }
        _ => None,
    };
    let mut fresh = AgentSession::new(scope_key, scope_kind);
    // Same change scope as the donor — carry facts so empty-donor bootstrap
    // can resolve without waiting for the next project refresh tick.
    if let Some(d) = donor {
        fresh.scope_facts = d.scope_facts.clone();
    }
    if let Some(actions) = donor_actions {
        fresh.inherited_next_actions = Some(actions);
    }
    fresh.refresh_next_actions(true);
    fresh
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
    use crate::widget::text_edit::BlockKind;

    /// Build the scope orientation blurb the way `send_prompt_text` does, so
    /// the priming-body tests assert against the real hook output.
    fn scope_blurb(kind: ScopeKind, key: &str) -> String {
        use duckchat::ContextHook;
        crate::scope::CurrentScopeHook
            .compute(&crate::scope::SessionScope {
                kind,
                scope_key: key.into(),
                change_facts: None,
            })
            .expect("scope hook always produces orientation")
            .text
    }

    // @spec chat/default-prompts Next-action list: Empty exploration session seeds explore
    #[test]
    fn empty_exploration_session_seeds_explore() {
        // GIVEN an empty exploration session transcript
        let mut ax = AgentSession::new("exploration-1".into(), ScopeKind::Exploration);
        assert!(ax.session.messages.is_empty());
        // WHEN the next-action list is built
        ax.refresh_next_actions(false);
        // THEN the list is exactly the explore stage command in empty-send form
        assert_eq!(ax.next_actions.len(), 1);
        assert_eq!(ax.next_actions[0].send, "/ds-explore");
    }

    // @spec chat/default-prompts Next-action list: Empty change session with unfinished steps seeds apply
    #[test]
    fn empty_change_session_with_unfinished_steps_seeds_apply() {
        // GIVEN an empty change session with unfinished steps (first lifecycle = apply)
        let mut ax = AgentSession::new("foo".into(), ScopeKind::Change);
        ax.scope_facts = Some(crate::area::change::ChangeScopeFacts {
            phase: "implementing steps",
            steps_done: 0,
            step_count: 2,
            active_step_tasks: Some((0, 3)),
            next_command: Some("ds-apply".into()),
            current_review: None,
        });
        assert!(ax.session.messages.is_empty());
        // WHEN the next-action list is built
        ax.refresh_next_actions(false);
        // THEN the list is exactly the apply stage command in empty-send form
        assert_eq!(ax.next_actions.len(), 1);
        assert_eq!(ax.next_actions[0].send, "/ds-apply");
    }

    // @spec chat/default-prompts New-session next-action inheritance: New change session inherits active session next actions
    #[test]
    fn new_change_session_inherits_active_session_next_actions() {
        // GIVEN change multi-session chat + active session with two next actions
        let mut donor = AgentSession::new("foo".into(), ScopeKind::Change);
        donor.next_actions = vec![
            crate::meta_card::NextAction {
                send: "/ds-spec".into(),
                reason: Some("write specs".into()),
            },
            crate::meta_card::NextAction {
                send: "confirm".into(),
                reason: None,
            },
        ];
        donor.next_action_idx = 0;
        let mut ix = InteractionState::default();
        ix.sessions.push(donor);
        ix.active_session = 0;
        // WHEN a new chat session is created for that change
        let fresh = new_session_with_inherited_next_actions(&ix, "foo".into(), ScopeKind::Change);
        // THEN the new session's list matches the donor tokens; transcript empty
        assert!(fresh.session.messages.is_empty());
        assert_eq!(fresh.next_actions.len(), 2);
        assert_eq!(fresh.next_actions[0].send, "/ds-spec");
        assert_eq!(fresh.next_actions[1].send, "confirm");
        assert!(fresh.inherited_next_actions.is_some());
    }

    // @spec chat/default-prompts New-session next-action inheritance: New change session with empty donor keeps bootstrap behavior
    #[test]
    fn new_change_session_with_empty_donor_keeps_bootstrap_behavior() {
        // GIVEN change multi-session + active empty next_actions + lifecycle option
        let mut donor = AgentSession::new("foo".into(), ScopeKind::Change);
        donor.next_actions.clear();
        donor.scope_facts = Some(crate::area::change::ChangeScopeFacts {
            phase: "proposal",
            steps_done: 0,
            step_count: 0,
            active_step_tasks: None,
            next_command: Some("ds-propose".into()),
            current_review: None,
        });
        let mut ix = InteractionState::default();
        ix.sessions.push(donor);
        ix.active_session = 0;
        // WHEN a new chat session is created for that change
        let fresh = new_session_with_inherited_next_actions(&ix, "foo".into(), ScopeKind::Change);
        // THEN bootstrap only (no inheritance)
        assert!(fresh.inherited_next_actions.is_none());
        assert_eq!(fresh.next_actions.len(), 1);
        assert_eq!(fresh.next_actions[0].send, "/ds-propose");
        assert!(fresh.session.messages.is_empty());
    }

    // @spec chat/default-prompts New-session next-action inheritance: Inherited list starts at first action
    #[test]
    fn inherited_list_starts_at_first_action() {
        // GIVEN donor with ≥2 next actions and active index not first
        let mut donor = AgentSession::new("foo".into(), ScopeKind::Change);
        donor.next_actions = vec![
            crate::meta_card::NextAction {
                send: "/ds-spec".into(),
                reason: None,
            },
            crate::meta_card::NextAction {
                send: "/ds-design".into(),
                reason: None,
            },
        ];
        donor.next_action_idx = 1;
        let mut ix = InteractionState::default();
        ix.sessions.push(donor);
        ix.active_session = 0;
        // WHEN a new chat session is created for that change
        let fresh = new_session_with_inherited_next_actions(&ix, "foo".into(), ScopeKind::Change);
        // THEN empty submit sends the first inherited token
        assert_eq!(fresh.next_action_idx, 0);
        assert_eq!(
            crate::default_prompts::next_empty_submit_text(
                false,
                &fresh.next_actions,
                fresh.next_action_idx
            )
            .as_deref(),
            Some("/ds-spec")
        );
    }

    // @spec chat/default-prompts Agent input hints gate: Empty-session next actions remain when agent input hints disabled
    #[test]
    fn empty_session_next_actions_remain_when_agent_input_hints_disabled() {
        // GIVEN agent input hints disabled + empty session + first lifecycle option
        let agent_input_hints = false;
        let mut ax = AgentSession::new("exploration-1".into(), ScopeKind::Exploration);
        assert!(ax.session.messages.is_empty());
        // WHEN the next-action list is built
        ax.refresh_next_actions(false);
        // THEN the list is exactly that single lifecycle option in empty-send form
        // (hints only gate oneshot — not next-action bootstrap)
        assert!(!agent_input_hints);
        assert!(ax.session_oneshot_prompts(agent_input_hints).is_empty());
        assert_eq!(ax.next_actions.len(), 1);
        assert_eq!(ax.next_actions[0].send, "/ds-explore");
    }

    /// @spec harness/selection Default model resolution: An empty cascade resolves to grok-4.5
    #[test]
    fn empty_cascade_resolves_to_grok_4_5() {
        // GIVEN neither a per-chat pin nor a project default.
        // WHEN the model for a turn is resolved.
        let resolved = resolve_turn_model(None, None);
        // THEN the resolved model is grok-4.5 on the grok harness.
        assert_eq!(resolved.harness, "grok");
        assert_eq!(resolved.model, "grok-4.5");
    }

    /// @spec harness/selection Default model resolution: A per-chat pin overrides a project default
    #[test]
    fn per_chat_pin_overrides_project_default() {
        // GIVEN a per-chat pin and a different project default.
        let pin = ModelRef::new("claude-code", "opus");
        let project_default = ModelRef::new("grok", "grok-4.5");
        // WHEN the model for a turn is resolved.
        let resolved = resolve_turn_model(Some(&pin), Some(&project_default));
        // THEN the resolved model is the per-chat pin.
        assert_eq!(resolved, pin);
    }

    /// @spec session/scope Reliable first-turn delivery: The first turn's message body carries the scope orientation
    #[test]
    fn priming_body_carries_scope_orientation() {
        let blurb = scope_blurb(ScopeKind::Change, "foo");
        let body = assemble_priming_body(Some("AGENTS conventions"), Some(&blurb));
        assert!(
            body.contains(&blurb),
            "first-turn body must carry the scope orientation: {body}"
        );
        assert!(
            body.contains("single dot"),
            "priming body keeps the single-dot-ack instruction: {body}"
        );
        // A brand-new session is the one that gets primed.
        assert!(should_prime(None, false));
    }

    /// @spec session/scope Reliable first-turn delivery: Orientation is present when the project has no AGENTS.md
    #[test]
    fn priming_body_present_without_agents_md() {
        let blurb = scope_blurb(ScopeKind::Change, "foo");
        // No AGENTS.md → still primed, and the orientation still rides the body.
        let body = assemble_priming_body(None, Some(&blurb));
        assert!(
            body.contains(&blurb),
            "orientation must be present even with no AGENTS.md: {body}"
        );
        assert!(
            body.contains(PATH_REFERENCE_NOTE),
            "path-reference note keeps the body non-empty without AGENTS.md: {body}"
        );
        assert!(
            should_prime(None, false),
            "a fresh session is primed regardless of AGENTS.md presence"
        );
    }

    /// @spec session/scope Reliable first-turn delivery: A resumed session does not repeat the orientation
    #[test]
    fn resumed_session_is_not_re_primed() {
        // A resumable Claude session id means the orientation is already in the
        // session's history — do not prime again.
        assert!(!should_prime(Some("claude-session-123"), false));
        // Likewise a session that already has prior messages.
        assert!(!should_prime(None, true));
        // Only the brand-new session (no id, no messages) is primed.
        assert!(should_prime(None, false));
    }

    #[test]
    fn session_id_resumable_only_for_the_owning_harness() {
        let mut ax = AgentSession::new("foo".to_string(), ScopeKind::Change);
        ax.session.agent_session_id = Some("sess-abc".to_string());
        ax.session.session_harness = Some("claude-code".to_string());

        // Pinned to the harness that produced the id → resume it.
        ax.session.selected_model = Some(ModelRef::new("claude-code", "opus"));
        assert_eq!(ax.resumable_session_id(), Some("sess-abc"));

        // Switched to grok: the Claude id is foreign and would fail
        // `session/load`, so the turn must start a fresh agent session.
        ax.session.selected_model = Some(ModelRef::new("grok", "grok-4.5"));
        assert_eq!(ax.resumable_session_id(), None);
    }

    #[test]
    fn legacy_session_id_without_owner_is_claude_code() {
        // A session saved before harnesses existed carries an id but no owner.
        let mut ax = AgentSession::new("foo".to_string(), ScopeKind::Change);
        ax.session.agent_session_id = Some("legacy-id".to_string());
        ax.session.session_harness = None;
        ax.session.selected_model = Some(ModelRef::new("claude-code", "opus"));
        // It resumes under Claude Code, the only backend that could have made it.
        assert_eq!(ax.resumable_session_id(), Some("legacy-id"));
    }

    #[test]
    fn recover_from_lost_session_clears_dead_resume_id() {
        // After session/load FS_NOT_FOUND the stored id must not stick around
        // and wedge every subsequent send. Without an agent handle we only
        // clear + stop streaming (retry needs a handle).
        let mut ax = AgentSession::new("exploration-1".to_string(), ScopeKind::Exploration);
        ax.session.agent_session_id = Some("019f489b-dead".to_string());
        ax.session.session_harness = Some("grok".to_string());
        ax.session.selected_model = Some(ModelRef::new("grok", "grok-4.5"));
        ax.session.is_streaming = true;
        ax.session.messages.push(ChatMessage {
            role: Role::User,
            content: vec![ContentBlock::Text("hello".into())],
            timestamp: String::new(),
            is_priming: false,
        });
        assert!(ax.resumable_session_id().is_some());

        let hl = SyntaxHighlighter::new();
        recover_from_lost_session(&mut ax, &hl);

        assert!(ax.session.agent_session_id.is_none());
        assert!(ax.session.session_harness.is_none());
        assert!(ax.resumable_session_id().is_none());
        assert!(!ax.session.is_streaming);
        // Transcript is left intact for the next send / handle-backed retry.
        assert_eq!(ax.session.messages.len(), 1);
    }

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
        assert!(
            out.contains("File: src/main.rs (12:5–24:1)"),
            "header: {out}"
        );
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
        assert!(
            out.contains("Chat excerpt: User message (block 3"),
            "header: {out}"
        );
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

    // ── chat/stream-ui: session apply + bounded materialize ───────────────

    fn streaming_session() -> AgentSession {
        let mut ax = AgentSession::new("stream-ui-test".into(), ScopeKind::Change);
        ax.session.is_streaming = true;
        ax
    }

    // @spec chat/stream-ui Session apply before materialize: Content deltas accumulate on the session without materialization
    #[test]
    fn content_deltas_accumulate_without_materialization() {
        let mut ax = streaming_session();
        let blocks_before = ax.chat_blocks.len();

        let ks1 = apply_answer_content_delta(&mut ax.session, "hello");
        ax.chat_ui_dirty = true;
        assert!(!ks1);
        assert!(!should_materialize_chat_ui(
            &crate::agent::AgentEvent::ContentDelta {
                text: "hello".into()
            },
            true,
            ks1,
        ));

        let ks2 = apply_answer_content_delta(&mut ax.session, " world");
        ax.chat_ui_dirty = true;
        assert!(!ks2);

        assert_eq!(ax.session.pending_text, "hello world");
        // Materialize was never called — UI still empty of the live answer.
        assert_eq!(ax.chat_blocks.len(), blocks_before);
        assert!(ax.chat_ui_dirty);
    }

    // @spec chat/stream-ui Session apply before materialize: Reasoning deltas accumulate on the session without materialization
    #[test]
    fn reasoning_deltas_accumulate_without_materialization() {
        let mut ax = streaming_session();
        let blocks_before = ax.chat_blocks.len();

        let ks1 = apply_reasoning_content_delta(&mut ax.session, "think");
        ax.chat_ui_dirty = true;
        assert!(!ks1);
        assert!(!should_materialize_chat_ui(
            &crate::agent::AgentEvent::ReasoningDelta {
                text: "think".into()
            },
            true,
            ks1,
        ));

        let ks2 = apply_reasoning_content_delta(&mut ax.session, " more");
        ax.chat_ui_dirty = true;
        assert!(!ks2);

        assert_eq!(ax.session.pending_reasoning, "think more");
        assert_eq!(ax.chat_blocks.len(), blocks_before);
        assert!(ax.chat_ui_dirty);
    }

    // @spec chat/stream-ui Bounded materialization while streaming: Pure content deltas alone do not materialize the chat UI
    #[test]
    fn pure_content_deltas_alone_do_not_materialize() {
        assert!(!should_materialize_chat_ui(
            &crate::agent::AgentEvent::ContentDelta {
                text: "x".into()
            },
            true,
            false,
        ));
        assert!(!should_materialize_chat_ui(
            &crate::agent::AgentEvent::ReasoningDelta {
                text: "y".into()
            },
            true,
            false,
        ));
        // Kind switch is structural even for content deltas.
        assert!(should_materialize_chat_ui(
            &crate::agent::AgentEvent::ContentDelta {
                text: "x".into()
            },
            true,
            true,
        ));
        // Structural events always materialize.
        assert!(is_structural_chat_event(&crate::agent::AgentEvent::ToolUse {
            id: "1".into(),
            name: "Bash".into(),
            input: "ls".into(),
        }));
        assert!(is_structural_chat_event(
            &crate::agent::AgentEvent::TurnComplete
        ));
    }

    // @spec chat/stream-ui Bounded materialization while streaming: Stream UI tick materializes accumulated session answer text into the chat UI
    #[test]
    fn stream_ui_tick_materializes_accumulated_answer() {
        let mut ax = streaming_session();
        ax.stick_to_bottom = true;
        apply_answer_content_delta(&mut ax.session, "batch one");
        apply_answer_content_delta(&mut ax.session, " batch two");
        ax.chat_ui_dirty = true;
        assert_eq!(ax.session.pending_text, "batch one batch two");
        assert!(ax.chat_blocks.is_empty());
        assert!(should_materialize_on_stream_tick(
            ax.session.is_streaming,
            ax.chat_ui_dirty,
            ax.stick_to_bottom,
        ));

        // StreamTick path: materialize while dirty + streaming + stick.
        let hl = SyntaxHighlighter::new();
        materialize_chat_ui(&mut ax, &hl);

        assert!(!ax.chat_ui_dirty);
        let joined: String = ax
            .chat_blocks
            .iter()
            .flat_map(|b| b.lines.iter().cloned())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("batch one batch two"),
            "materialized UI must include accumulated pending answer: {joined:?}"
        );
    }

    // @spec chat/stream-ui Bounded materialization while streaming: Stream UI tick skips materialize while scrolled up in history
    #[test]
    fn stream_ui_tick_skips_materialize_when_scrolled_up() {
        let mut ax = streaming_session();
        ax.stick_to_bottom = false;
        apply_answer_content_delta(&mut ax.session, "while reading history");
        ax.chat_ui_dirty = true;
        assert!(!should_materialize_on_stream_tick(
            ax.session.is_streaming,
            ax.chat_ui_dirty,
            ax.stick_to_bottom,
        ));
        // Session still holds the text; UI not rebuilt.
        assert_eq!(ax.session.pending_text, "while reading history");
        assert!(ax.chat_blocks.is_empty());
        assert!(ax.chat_ui_dirty);
    }

    // @spec chat/stream-ui Bounded materialization while streaming: Re-sticking to bottom materializes deferred content
    #[test]
    fn restick_to_bottom_materializes_deferred_content() {
        let mut ax = streaming_session();
        ax.stick_to_bottom = false;
        apply_answer_content_delta(&mut ax.session, "deferred while up");
        ax.chat_ui_dirty = true;
        assert!(ax.chat_blocks.is_empty());

        // User scrolls back to bottom → stick re-engages → materialize.
        ax.stick_to_bottom = true;
        let hl = SyntaxHighlighter::new();
        if ax.stick_to_bottom && ax.chat_ui_dirty && ax.session.is_streaming {
            materialize_chat_ui(&mut ax, &hl);
        }

        assert!(!ax.chat_ui_dirty);
        let joined: String = ax
            .chat_blocks
            .iter()
            .flat_map(|b| b.lines.iter().cloned())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("deferred while up"),
            "re-stick must paint deferred answer: {joined:?}"
        );
    }

    // @spec chat/stream-ui Bounded materialization while streaming: Tool use materializes the chat UI immediately with an Activity row
    #[test]
    fn tool_use_materializes_immediately_with_activity_row() {
        let mut ax = streaming_session();
        assert!(should_materialize_chat_ui(
            &crate::agent::AgentEvent::ToolUse {
                id: "t1".into(),
                name: "Bash".into(),
                input: "echo hi".into(),
            },
            true,
            false,
        ));

        flush_all_pending(&mut ax.session);
        ax.session.messages.push(ChatMessage {
            role: Role::Assistant,
            content: vec![ContentBlock::ToolUse {
                id: "t1".into(),
                name: "Bash".into(),
                input: "echo hi".into(),
            }],
            timestamp: String::new(),
            is_priming: false,
        });
        ax.chat_ui_dirty = true;

        let hl = SyntaxHighlighter::new();
        materialize_chat_ui(&mut ax, &hl);

        assert!(!ax.chat_ui_dirty);
        let has_activity = ax
            .chat_blocks
            .iter()
            .any(|b| matches!(b.kind, BlockKind::Activity | BlockKind::ToolUse));
        assert!(
            has_activity,
            "expected an Activity/tool block after tool use materialize; blocks={:?}",
            ax.chat_blocks.iter().map(|b| &b.kind).collect::<Vec<_>>()
        );
    }

    // @spec chat/stream-ui Bounded materialization while streaming: Turn complete materializes the final answer immediately
    #[test]
    fn turn_complete_materializes_final_answer_immediately() {
        let mut ax = streaming_session();
        apply_answer_content_delta(&mut ax.session, "final answer body");
        ax.chat_ui_dirty = true;

        assert!(should_materialize_chat_ui(
            &crate::agent::AgentEvent::TurnComplete,
            true,
            false,
        ));

        // TurnComplete apply: flush pending, stop streaming, materialize now.
        flush_all_pending(&mut ax.session);
        ax.session.is_streaming = false;
        let hl = SyntaxHighlighter::new();
        materialize_chat_ui(&mut ax, &hl);

        assert!(!ax.chat_ui_dirty);
        assert!(!ax.session.is_streaming);
        let joined: String = ax
            .chat_blocks
            .iter()
            .flat_map(|b| b.lines.iter().cloned())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("final answer body"),
            "final answer must paint without waiting for another tick: {joined:?}"
        );
    }

    // ── chat/stream-ui: answer draft across thought ───────────────────────

    fn committed_answer_texts(session: &ChatSession) -> Vec<String> {
        session
            .messages
            .iter()
            .filter(|m| m.role == Role::Assistant)
            .flat_map(|m| m.content.iter())
            .filter_map(|b| match b {
                ContentBlock::Text(t) => Some(t.clone()),
                _ => None,
            })
            .collect()
    }

    // @spec chat/stream-ui Answer draft across thought: Reasoning leaves the open answer uncommitted
    #[test]
    fn reasoning_leaves_open_answer_uncommitted() {
        let mut ax = streaming_session();
        apply_answer_content_delta(&mut ax.session, "draft answer");
        let msg_count = ax.session.messages.len();

        let ks = apply_reasoning_content_delta(&mut ax.session, "rethink");
        assert!(ks, "answer→reasoning is a structural channel switch");
        assert_eq!(ax.session.pending_text, "draft answer");
        assert_eq!(ax.session.pending_reasoning, "rethink");
        assert_eq!(ax.session.messages.len(), msg_count);
        assert!(
            committed_answer_texts(&ax.session).is_empty(),
            "reasoning must not commit the open answer draft"
        );
    }

    // @spec chat/stream-ui Answer draft across thought: Answer after reasoning replaces the live draft
    #[test]
    fn answer_after_reasoning_replaces_live_draft() {
        let mut ax = streaming_session();
        apply_answer_content_delta(&mut ax.session, "first body");
        apply_reasoning_content_delta(&mut ax.session, "think again");

        let ks = apply_answer_content_delta(&mut ax.session, "second body");
        assert!(ks, "reasoning→answer is a structural channel switch");
        assert_eq!(ax.session.pending_text, "second body");
        assert!(!ax.session.pending_text.contains("first body"));
        assert!(ax.session.pending_reasoning.is_empty());
        // Prior draft was replaced, not committed.
        assert!(
            committed_answer_texts(&ax.session).is_empty(),
            "replaced draft must not leave a committed Text for the first body"
        );
        // Reasoning from the interlude is committed when answer resumes.
        assert!(
            ax.session.messages.iter().any(|m| {
                m.content
                    .iter()
                    .any(|b| matches!(b, ContentBlock::Reasoning(t) if t == "think again"))
            }),
            "pending reasoning should flush when answer resumes"
        );
    }

    // @spec chat/stream-ui Answer draft across thought: Tool use commits the open answer draft
    #[test]
    fn tool_use_commits_open_answer_draft() {
        let mut ax = streaming_session();
        apply_answer_content_delta(&mut ax.session, "status before tools");

        // Mirror ToolUse handling: flush drafts, then record the tool.
        flush_all_pending(&mut ax.session);
        ax.session.messages.push(ChatMessage {
            role: Role::Assistant,
            content: vec![ContentBlock::ToolUse {
                id: "t1".into(),
                name: "Read".into(),
                input: "f".into(),
            }],
            timestamp: String::new(),
            is_priming: false,
        });

        assert!(ax.session.pending_text.is_empty());
        assert_eq!(
            committed_answer_texts(&ax.session),
            vec!["status before tools".to_string()]
        );
    }

    // @spec chat/stream-ui Bounded materialization while streaming: Answer-to-reasoning channel switch materializes without committing the answer
    #[test]
    fn answer_to_reasoning_channel_switch_materializes_without_commit() {
        let mut ax = streaming_session();
        apply_answer_content_delta(&mut ax.session, "open draft");

        let ks = apply_reasoning_content_delta(&mut ax.session, "thinking…");
        assert!(ks);
        assert!(should_materialize_chat_ui(
            &crate::agent::AgentEvent::ReasoningDelta {
                text: "thinking…".into()
            },
            true,
            ks,
        ));
        assert_eq!(ax.session.pending_text, "open draft");
        assert!(
            committed_answer_texts(&ax.session).is_empty(),
            "channel switch materializes without committing the answer draft"
        );
    }

    /// Drive answer → thought → answer replace `n` times (each ends with a draft).
    fn thrash_replaces(session: &mut ChatSession, n: u32) {
        apply_answer_content_delta(session, "body-0");
        for i in 1..=n {
            apply_reasoning_content_delta(session, "think");
            apply_answer_content_delta(session, &format!("body-{i}"));
        }
    }

    // @spec chat/stream-ui Answer thrash budget: Exceeding the budget cancels and keeps the last draft
    #[test]
    fn exceeding_budget_trips_thrash_keeps_last_draft() {
        let mut ax = streaming_session();
        // Use the full allowed replacement budget → draft is body-{budget}.
        thrash_replaces(&mut ax.session, ANSWER_REPLACE_BUDGET);
        let last_allowed = format!("body-{ANSWER_REPLACE_BUDGET}");
        assert_eq!(ax.session.pending_text, last_allowed);
        assert!(!ax.session.answer_thrash_tripped);
        assert_eq!(ax.session.answer_replace_count, ANSWER_REPLACE_BUDGET);

        // Next replace attempt: trip without replacing the last complete draft.
        apply_reasoning_content_delta(&mut ax.session, "think again");
        let ks = apply_answer_content_delta(&mut ax.session, "body-should-not-apply");
        assert!(ks);
        assert!(ax.session.answer_thrash_tripped);
        assert_eq!(ax.session.pending_text, last_allowed);

        // Caller settles: flush draft + stop notice (mirrors main thrash path).
        on_answer_thrash_trip(&mut ax.session);
        assert!(ax.session.pending_text.is_empty());
        assert_eq!(
            committed_answer_texts(&ax.session),
            vec![last_allowed]
        );
        assert!(
            ax.session.messages.iter().any(|m| {
                m.role == Role::System
                    && m.content.iter().any(|b| {
                        matches!(b, ContentBlock::Text(t) if t == ANSWER_THRASH_STOP_NOTICE)
                    })
            }),
            "stop notice must be present as a system message"
        );

        // Further deltas are dropped.
        apply_answer_content_delta(&mut ax.session, "late thrash");
        apply_reasoning_content_delta(&mut ax.session, "late think");
        assert!(ax.session.pending_text.is_empty());
        assert!(ax.session.pending_reasoning.is_empty());
    }

    // @spec chat/stream-ui Answer thrash budget: Tool use resets the thrash budget
    #[test]
    fn tool_use_resets_thrash_budget() {
        let mut ax = streaming_session();
        thrash_replaces(&mut ax.session, ANSWER_REPLACE_BUDGET);
        assert_eq!(ax.session.answer_replace_count, ANSWER_REPLACE_BUDGET);

        // Tool boundary: commit draft and reset thrash (mirrors ToolUse handling).
        flush_all_pending(&mut ax.session);
        reset_answer_thrash(&mut ax.session);
        ax.session.messages.push(ChatMessage {
            role: Role::Assistant,
            content: vec![ContentBlock::ToolUse {
                id: "t1".into(),
                name: "Read".into(),
                input: "f".into(),
            }],
            timestamp: String::new(),
            is_priming: false,
        });

        assert_eq!(ax.session.answer_replace_count, 0);
        assert!(!ax.session.answer_thrash_tripped);

        // A new answer-after-thought replace after tools must not trip solely
        // from the pre-tool thrash count (still within a fresh budget).
        apply_answer_content_delta(&mut ax.session, "after-tool-1");
        apply_reasoning_content_delta(&mut ax.session, "think");
        apply_answer_content_delta(&mut ax.session, "after-tool-2");
        assert!(!ax.session.answer_thrash_tripped);
        assert_eq!(ax.session.pending_text, "after-tool-2");
        assert_eq!(ax.session.answer_replace_count, 1);
        assert!(ax.session.answer_replace_count <= ANSWER_REPLACE_BUDGET);
    }

    // ── chat/cancel-resync: draft capture on cancellation ─────────────────

    // @spec chat/cancel-resync Draft capture on cancellation: Thrash trip captures the kept draft
    #[test]
    fn thrash_trip_captures_the_kept_draft() {
        let mut ax = streaming_session();
        // GIVEN a streaming turn whose in-flight answer draft is non-empty.
        thrash_replaces(&mut ax.session, ANSWER_REPLACE_BUDGET);
        let kept = format!("body-{ANSWER_REPLACE_BUDGET}");
        assert_eq!(ax.session.pending_text, kept);

        // WHEN the answer-thrash budget trips and the turn is cancelled
        // (mirrors the main path: trip settle before cancelling the agent).
        apply_reasoning_content_delta(&mut ax.session, "think again");
        apply_answer_content_delta(&mut ax.session, "over-budget rewrite");
        assert!(ax.session.answer_thrash_tripped);
        on_answer_thrash_trip(&mut ax.session);

        // THEN the session's unsynced draft equals the kept draft.
        assert_eq!(ax.session.unsynced_draft.as_deref(), Some(kept.as_str()));
    }

    // @spec chat/cancel-resync Draft capture on cancellation: User cancel captures the in-flight draft
    #[test]
    fn user_cancel_captures_the_in_flight_draft() {
        let mut ax = streaming_session();
        // GIVEN a streaming turn whose in-flight answer draft is non-empty.
        apply_answer_content_delta(&mut ax.session, "half-written reply");
        assert_eq!(ax.session.pending_text, "half-written reply");

        // WHEN the user cancels the turn (mirrors the CancelPressed arm,
        // which captures before signalling the agent handle).
        capture_unsynced_draft(&mut ax.session);

        // THEN the session's unsynced draft equals that draft.
        assert_eq!(
            ax.session.unsynced_draft.as_deref(),
            Some("half-written reply")
        );
    }

    // @spec chat/cancel-resync Draft capture on cancellation: Cancellation with no in-flight draft records nothing
    #[test]
    fn cancellation_with_no_in_flight_draft_records_nothing() {
        let mut ax = streaming_session();
        // GIVEN a streaming turn whose answer text was committed at a tool
        // boundary AND no answer text has streamed since.
        apply_answer_content_delta(&mut ax.session, "committed before tools");
        flush_all_pending(&mut ax.session);
        assert!(ax.session.pending_text.is_empty());

        // WHEN the turn is cancelled.
        capture_unsynced_draft(&mut ax.session);

        // THEN the session has no unsynced draft.
        assert_eq!(ax.session.unsynced_draft, None);
    }

    // @spec chat/cancel-resync Draft capture on cancellation: Deltas arriving after cancel are part of the captured draft
    #[test]
    fn deltas_arriving_after_cancel_are_part_of_the_captured_draft() {
        let mut ax = streaming_session();
        // GIVEN a streaming turn cancelled by the user with a non-empty
        // in-flight answer draft (capture at cancel press).
        apply_answer_content_delta(&mut ax.session, "first half");
        capture_unsynced_draft(&mut ax.session);
        ax.cancel_in_flight = true;

        // WHEN further answer deltas arrive before the turn ends (the
        // TurnComplete handler re-captures while cancel is in flight).
        apply_answer_content_delta(&mut ax.session, " and late tail");
        if ax.cancel_in_flight {
            capture_unsynced_draft(&mut ax.session);
            ax.cancel_in_flight = false;
        }

        // THEN the session's unsynced draft includes those deltas.
        assert_eq!(
            ax.session.unsynced_draft.as_deref(),
            Some("first half and late tail")
        );
    }

    // ── chat/cancel-resync: resync reminder on next send ──────────────────

    // @spec chat/cancel-resync Resync reminder on next send: The next send carries the draft after the user's text
    #[test]
    fn next_send_carries_the_draft_after_the_users_text() {
        let mut ax = streaming_session();
        // GIVEN a session holding an unsynced draft.
        ax.session.unsynced_draft = Some("outline gate preview".into());

        // WHEN a prompt is sent on that session (mirrors send_prompt_text).
        let prompt = apply_resync_reminder("confirm".into(), &mut ax.session);

        // THEN the outgoing prompt begins with the user's text AND the
        // unsynced draft follows it.
        assert!(prompt.starts_with("confirm"), "user text must stay first: {prompt}");
        let user_pos = prompt.find("confirm").unwrap();
        let draft_pos = prompt
            .find("outline gate preview")
            .expect("draft must ride the prompt");
        assert!(draft_pos > user_pos);
    }

    // @spec chat/cancel-resync Resync reminder on next send: The reminder rides only one send
    #[test]
    fn reminder_rides_only_one_send() {
        let mut ax = streaming_session();
        // GIVEN a session holding an unsynced draft.
        ax.session.unsynced_draft = Some("outline gate preview".into());

        // WHEN two prompts are sent in sequence on that session.
        let first = apply_resync_reminder("confirm".into(), &mut ax.session);
        let second = apply_resync_reminder("next message".into(), &mut ax.session);

        // THEN only the first outgoing prompt carries the draft.
        assert!(first.contains("outline gate preview"));
        assert_eq!(second, "next message");
        assert_eq!(ax.session.unsynced_draft, None);
    }

    // @spec chat/cancel-resync Resync reminder on next send: A recovery resend carrying transcript history clears the draft without a reminder
    #[test]
    fn recovery_resend_clears_the_draft_without_a_reminder() {
        let mut ax = streaming_session();
        // GIVEN a session holding an unsynced draft, whose transcript keeps
        // that draft as a committed assistant message.
        ax.session.messages.push(ChatMessage {
            role: Role::Assistant,
            content: vec![ContentBlock::Text("kept outline draft".into())],
            timestamp: String::new(),
            is_priming: false,
        });
        ax.session.messages.push(ChatMessage {
            role: Role::User,
            content: vec![ContentBlock::Text("confirm".into())],
            timestamp: String::new(),
            is_priming: false,
        });
        ax.session.unsynced_draft = Some("kept outline draft".into());

        // WHEN a recovery send carries the transcript history in its prompt.
        let history_end = ax.session.messages.len() - 1;
        let prompt = build_recovery_prompt(&mut ax.session, history_end, "confirm");

        // THEN the session afterward holds no unsynced draft AND the
        // recovery prompt carries no resync reminder (the history preamble
        // already carries the kept draft).
        assert_eq!(ax.session.unsynced_draft, None);
        assert!(!prompt.contains("<system-reminder>"));
        assert!(prompt.contains("kept outline draft"));
    }

    // ── chat/stream-ui: settled + live editor refresh ─────────────────────

    #[test]
    fn plan_editor_refresh_suffix_and_reshape() {
        let a = vec!["hello".into()];
        let b = vec!["hello".into(), "world".into()];
        assert_eq!(
            plan_editor_refresh(&a, &a),
            EditorRefreshKind::Reuse
        );
        assert_eq!(
            plan_editor_refresh(&a, &b),
            EditorRefreshKind::InPlace { dirty_from: 1 }
        );
        assert_eq!(
            plan_editor_refresh(&["hel".into()], &["hello".into()]),
            EditorRefreshKind::InPlace { dirty_from: 0 }
        );
        assert_eq!(
            plan_editor_refresh(&["a".into(), "b".into()], &["a".into(), "c".into()]),
            EditorRefreshKind::FullRebuild
        );
    }

    // @spec chat/stream-ui Settled and live editor refresh: Unchanged settled block keeps its editor across materialize
    #[test]
    fn unchanged_settled_block_keeps_editor_across_materialize() {
        let mut ax = streaming_session();
        ax.session.messages.push(ChatMessage {
            role: Role::User,
            content: vec![ContentBlock::Text("user asks".into())],
            timestamp: String::new(),
            is_priming: false,
        });
        apply_answer_content_delta(&mut ax.session, "first");
        let hl = SyntaxHighlighter::new();
        materialize_chat_ui(&mut ax, &hl);

        let user_idx = ax
            .chat_blocks
            .iter()
            .position(|b| b.kind == BlockKind::User)
            .expect("user block");
        let user_version = ax.chat_editors[user_idx].highlight_version;
        // Mark the settled editor so a full replace would reset version to 0
        // only if we wrongly rebuild — reuse must preserve this value.
        ax.chat_editors[user_idx].highlight_version = 7;
        let user_version = 7.max(user_version);

        apply_answer_content_delta(&mut ax.session, " more");
        materialize_chat_ui(&mut ax, &hl);

        let user_idx = ax
            .chat_blocks
            .iter()
            .position(|b| b.kind == BlockKind::User)
            .expect("user block");
        assert_eq!(
            ax.chat_editors[user_idx].highlight_version, user_version,
            "settled user block must keep its editor (version) across live-answer growth"
        );
        assert_eq!(ax.chat_blocks[user_idx].lines, vec!["user asks".to_string()]);
    }

    // @spec chat/stream-ui Settled and live editor refresh: Suffix-growing live answer refreshes in place
    #[test]
    fn suffix_growing_live_answer_refreshes_in_place() {
        let mut ax = streaming_session();
        apply_answer_content_delta(&mut ax.session, "line one");
        let hl = SyntaxHighlighter::new();
        materialize_chat_ui(&mut ax, &hl);

        let ans_idx = ax
            .chat_blocks
            .iter()
            .position(|b| b.kind == BlockKind::Assistant)
            .expect("answer block");
        // Bump so a FullRebuild (fresh EditorState::new → version 0) is
        // distinguishable from InPlace (version + 1).
        ax.chat_editors[ans_idx].highlight_version = 3u64;
        let v_before: u64 = 3;

        apply_answer_content_delta(&mut ax.session, "\nline two");
        materialize_chat_ui(&mut ax, &hl);

        let ans_idx = ax
            .chat_blocks
            .iter()
            .position(|b| b.kind == BlockKind::Assistant)
            .expect("answer block");
        assert_eq!(
            ax.chat_editors[ans_idx].highlight_version,
            v_before.wrapping_add(1),
            "suffix growth must refresh in place (bump version), not EditorState::new"
        );
        let lines = &ax.chat_editors[ans_idx].lines;
        assert!(
            lines.iter().any(|l| l.contains("line two"))
                || lines.join("\n").contains("line two"),
            "live answer must include suffix: {lines:?}"
        );
        let _ = v_before;
    }

    // @spec chat/stream-ui Settled and live editor refresh: Block list reshape uses full rebuild for affected indices
    #[test]
    fn block_list_reshape_full_rebuild_for_affected_keeps_settled() {
        let mut ax = streaming_session();
        ax.session.messages.push(ChatMessage {
            role: Role::User,
            content: vec![ContentBlock::Text("user asks".into())],
            timestamp: String::new(),
            is_priming: false,
        });
        apply_answer_content_delta(&mut ax.session, "partial answer");
        let hl = SyntaxHighlighter::new();
        materialize_chat_ui(&mut ax, &hl);

        let user_idx = ax
            .chat_blocks
            .iter()
            .position(|b| b.kind == BlockKind::User)
            .expect("user");
        ax.chat_editors[user_idx].highlight_version = 9;

        // Structural reshape: flush answer, insert a tool, then more answer.
        flush_all_pending(&mut ax.session);
        ax.session.messages.push(ChatMessage {
            role: Role::Assistant,
            content: vec![ContentBlock::ToolUse {
                id: "t1".into(),
                name: "Bash".into(),
                input: "ls".into(),
            }],
            timestamp: String::new(),
            is_priming: false,
        });
        apply_answer_content_delta(&mut ax.session, "after tool");
        materialize_chat_ui(&mut ax, &hl);

        let user_idx = ax
            .chat_blocks
            .iter()
            .position(|b| b.kind == BlockKind::User)
            .expect("user still present");
        assert_eq!(
            ax.chat_editors[user_idx].highlight_version, 9,
            "unchanged earlier block keeps editor through reshape"
        );

        let has_activity = ax
            .chat_blocks
            .iter()
            .any(|b| matches!(b.kind, BlockKind::Activity | BlockKind::ToolUse));
        assert!(has_activity, "reshape must include Activity for the tool");

        let has_post_tool_answer = ax.chat_blocks.iter().any(|b| {
            b.kind == BlockKind::Assistant
                && (b.lines.join("\n").contains("after tool")
                    || b.lines.iter().any(|l| l.contains("after tool")))
        });
        assert!(
            has_post_tool_answer,
            "expected a live answer after the tool; blocks={:?}",
            ax.chat_blocks
                .iter()
                .map(|b| (b.kind, b.lines.clone()))
                .collect::<Vec<_>>()
        );
    }

    // @spec chat/fast-response Freeform while awaiting: Freeform submit completes the pending choice as a custom answer
    #[test]
    fn freeform_submit_completes_the_pending_choice_as_a_custom_answer() {
        use crate::fast_response::FastResponseSource;
        use duckchat::UserChoiceAnswer;

        // GIVEN awaiting a user choice with freeform text
        let source = FastResponseSource::UserChoice {
            correlation_id: 42,
        };
        // WHEN submit is planned
        let plan = plan_freeform_while_awaiting(true, &source, "ship later")
            .expect("freeform while awaiting plans a custom answer");
        assert_eq!(plan.correlation_id, Some(42));
        assert_eq!(plan.text, "ship later");
        // THEN custom answer (not cancelled) carries freeform text
        let answer = UserChoiceAnswer::Custom {
            text: plan.text.clone(),
        };
        assert!(matches!(
            &answer,
            UserChoiceAnswer::Custom { text } if text == "ship later"
        ));
        assert!(!matches!(answer, UserChoiceAnswer::Cancelled));
        // No separate user transcript message for custom answer activation.
        let session = crate::chat_store::ChatSession::new("change".into());
        assert!(
            !session
                .messages
                .iter()
                .any(|m| matches!(m.role, crate::chat_store::Role::User)),
            "custom answer must not invent a user message"
        );

        // Not awaiting → ordinary streaming path (no freeform plan).
        assert!(plan_freeform_while_awaiting(false, &source, "hi").is_none());
        // Empty freeform → no plan.
        assert!(plan_freeform_while_awaiting(true, &source, "  ").is_none());
    }

    /// Option activation discards typed freeform so it is not left for a later send.
    #[test]
    fn option_activation_clears_typed_composer_text() {
        use crate::fast_response::{self, FastResponsePick, FastResponseSource};
        use crate::scope::ScopeKind;

        let hl = SyntaxHighlighter::new();
        let mut ax = AgentSession::new("foo".into(), ScopeKind::Change);
        ax.is_awaiting_user = true;
        ax.fast_response = fast_response::from_user_choice(
            7,
            [("opt-a".into(), "Alpha".into())],
        );
        ax.chat_input = EditorState::new("partial freeform");
        assert!(!ax.chat_input.text().trim().is_empty());

        activate_fast_response(
            &mut ax,
            FastResponsePick::Option {
                id: "opt-a".into(),
            },
            &hl,
        );

        assert!(
            ax.chat_input.text().trim().is_empty(),
            "typed freeform must be cleared on chip pick"
        );
        assert!(!ax.is_awaiting_user);
        assert!(matches!(
            ax.fast_response.source,
            FastResponseSource::None
        ));
        assert!(
            !ax.session
                .messages
                .iter()
                .any(|m| matches!(m.role, crate::chat_store::Role::User)),
            "option activation must not invent a user message"
        );
    }

    // ── layout/content-chat-split ─────────────────────────────────────────

    // @spec layout/content-chat-split Uncustomized equal width: Default half of free space
    #[test]
    fn default_half_of_free_space() {
        // GIVEN an uncustomized panel + window with free space large enough for half > min
        let window_w = theme::DEFAULT_WINDOW_WIDTH;
        let free = free_content_chat_width(window_w);
        assert!(
            free / 2.0 > interaction_toggle::MIN_PANEL_WIDTH,
            "fixture window must allow a half above min"
        );
        // WHEN the interaction column width is resolved
        let width = equal_interaction_width(window_w);
        // THEN width equals half of free space
        assert_eq!(width, free / 2.0);
        let ix = InteractionState::default();
        assert!(!ix.width_customized);
        assert_eq!(ix.width, equal_interaction_width(window_w));
    }

    // @spec layout/content-chat-split Uncustomized equal width: Resize rebalances to half free space
    #[test]
    fn resize_rebalances_to_half_free_space() {
        // GIVEN an uncustomized panel
        let mut ix = InteractionState::default();
        assert!(!ix.width_customized);
        // WHEN the window width changes to a new size still above min half
        let new_w = 1600.0;
        rebalance_uncustomized(&mut ix, new_w);
        // THEN width equals half free for the new window
        assert_eq!(ix.width, equal_interaction_width(new_w));
        assert_eq!(ix.width, free_content_chat_width(new_w) / 2.0);
    }

    // @spec layout/content-chat-split Uncustomized equal width: Half floors at minimum panel width
    #[test]
    fn half_floors_at_minimum_panel_width() {
        // GIVEN free space less than twice the minimum panel width
        // free = W - SIDEBAR - 1 - LIST - 1 - HANDLE; want free < 2 * MIN
        let fixed = theme::SIDEBAR_WIDTH
            + 1.0
            + theme::LIST_COLUMN_WIDTH
            + 1.0
            + interaction_toggle::HANDLE_WIDTH;
        let window_w = fixed + interaction_toggle::MIN_PANEL_WIDTH; // free = MIN < 2*MIN
        assert!(free_content_chat_width(window_w) < 2.0 * interaction_toggle::MIN_PANEL_WIDTH);
        // WHEN width is resolved
        let width = equal_interaction_width(window_w);
        // THEN width equals the minimum panel width
        assert_eq!(width, interaction_toggle::MIN_PANEL_WIDTH);
    }

    // @spec layout/content-chat-split Uncustomized equal width: Half may exceed the old fixed max width
    #[test]
    fn half_may_exceed_the_old_fixed_max_width() {
        // GIVEN free space more than twice 800
        let fixed = theme::SIDEBAR_WIDTH
            + 1.0
            + theme::LIST_COLUMN_WIDTH
            + 1.0
            + interaction_toggle::HANDLE_WIDTH;
        let window_w = fixed + 1601.0; // free > 1600 → half > 800
        assert!(free_content_chat_width(window_w) > 1600.0);
        // WHEN width is resolved
        let width = equal_interaction_width(window_w);
        // THEN half free and > 800
        assert_eq!(width, free_content_chat_width(window_w) / 2.0);
        assert!(width > 800.0);
    }

    // @spec layout/content-chat-split Grip customization: First grip width change locks absolute width
    #[test]
    fn first_grip_width_change_locks_absolute_width() {
        // GIVEN an uncustomized panel
        let mut ix = InteractionState::default();
        assert!(!ix.width_customized);
        let hl = SyntaxHighlighter::new();
        // WHEN the grip sets an absolute width
        let chosen = 350.0;
        update(
            &mut ix,
            Msg::Handle(interaction_toggle::HandleMsg::SetWidth(chosen)),
            &hl,
            false,
        );
        // THEN customized with that absolute width
        assert!(ix.width_customized);
        assert_eq!(ix.width, chosen);
        assert!(!ix.content_collapsed);
    }

    // @spec layout/content-chat-split Grip customization: Resize after lock keeps absolute width
    #[test]
    fn resize_after_lock_keeps_absolute_width() {
        // GIVEN a customized panel with remembered absolute width
        let mut ix = InteractionState::default();
        let hl = SyntaxHighlighter::new();
        let locked = 420.0;
        update(
            &mut ix,
            Msg::Handle(interaction_toggle::HandleMsg::SetWidth(locked)),
            &hl,
            false,
        );
        assert!(ix.width_customized);
        // WHEN window width changes
        rebalance_uncustomized(&mut ix, 1800.0);
        // THEN absolute width is kept
        assert_eq!(ix.width, locked);
    }

    // @spec layout/content-chat-split Content-hidden fill: Interaction column fills when content column is hidden
    #[test]
    fn interaction_column_fills_when_content_column_is_hidden() {
        // GIVEN a visible interaction panel with a remembered split width
        let remembered = 400.0;
        // AND the content column is not shown
        // WHEN the three-column area is laid out
        let size = interaction_column_size(false, remembered);
        // THEN the interaction column fills remaining width rather than fixed equal-split
        assert_eq!(size, InteractionColumnSize::Fill);
        // Contrast: with content shown, the remembered width is fixed
        assert_eq!(
            interaction_column_size(true, remembered),
            InteractionColumnSize::Fixed(remembered)
        );
    }

    // @spec layout/content-chat-split Content-hidden fill: No open tabs hides content column
    #[test]
    fn no_open_tabs_hides_content_column() {
        // GIVEN a three-column area with no open tabs and content not collapsed
        let has_tabs = false;
        let content_collapsed = false;
        // WHEN layout visibility is resolved
        let show = show_content_column(has_tabs, content_collapsed);
        // THEN content is not shown and interaction fills
        assert!(!show);
        assert_eq!(
            interaction_column_size(show, 400.0),
            InteractionColumnSize::Fill
        );
    }

    // @spec layout/content-chat-split Content-hidden fill: Opening a tab restores content column
    #[test]
    fn opening_a_tab_restores_content_column() {
        // GIVEN content hidden because there are no open tabs
        assert!(!show_content_column(false, false));
        // WHEN a list selection opens a tab (has_tabs becomes true)
        let show = show_content_column(true, false);
        // THEN content is shown and interaction uses fixed width
        assert!(show);
        let remembered = 437.0;
        assert_eq!(
            interaction_column_size(show, remembered),
            InteractionColumnSize::Fixed(remembered)
        );
    }

    // @spec layout/content-chat-split Grip customization: Open/close and content collapse do not lock
    #[test]
    fn open_close_and_content_collapse_do_not_lock() {
        // GIVEN an uncustomized panel
        let mut ix = InteractionState::default();
        let hl = SyntaxHighlighter::new();
        let start_w = ix.width;
        // WHEN closed and opened again
        update(
            &mut ix,
            Msg::Handle(interaction_toggle::HandleMsg::Toggle),
            &hl,
            false,
        );
        assert!(ix.visible);
        update(
            &mut ix,
            Msg::Handle(interaction_toggle::HandleMsg::Toggle),
            &hl,
            false,
        );
        assert!(!ix.visible);
        update(
            &mut ix,
            Msg::Handle(interaction_toggle::HandleMsg::Toggle),
            &hl,
            false,
        );
        // AND content collapsed and restored without a grip width change
        update(
            &mut ix,
            Msg::Handle(interaction_toggle::HandleMsg::SetCollapsed(true)),
            &hl,
            false,
        );
        assert!(ix.content_collapsed);
        update(
            &mut ix,
            Msg::Handle(interaction_toggle::HandleMsg::SetCollapsed(false)),
            &hl,
            false,
        );
        assert!(!ix.content_collapsed);
        // THEN still uncustomized and equal width for current (default) window
        assert!(!ix.width_customized);
        rebalance_uncustomized(&mut ix, theme::DEFAULT_WINDOW_WIDTH);
        assert_eq!(ix.width, equal_interaction_width(theme::DEFAULT_WINDOW_WIDTH));
        assert_eq!(ix.width, start_w);
    }

    // @spec layout/content-chat-split Uncustomized equal width: Panel created for a known window starts at half free space
    #[test]
    fn panel_created_for_a_known_window_starts_at_half_free_space() {
        // GIVEN a window width with free space large enough for half > min
        // AND that width is not the fixed default window size
        let window_w = 1800.0;
        assert_ne!(window_w, theme::DEFAULT_WINDOW_WIDTH);
        let free = free_content_chat_width(window_w);
        assert!(
            free / 2.0 > interaction_toggle::MIN_PANEL_WIDTH,
            "fixture window must allow a half above min"
        );
        // WHEN a new uncustomized panel is constructed for that window
        let ix = InteractionState::for_window(window_w);
        // THEN width equals half of free space for that window
        assert!(!ix.width_customized);
        assert_eq!(ix.width, equal_interaction_width(window_w));
        assert_eq!(ix.width, free / 2.0);
        // Contrast: Default is still tied to DEFAULT_WINDOW_WIDTH
        assert_eq!(
            InteractionState::default().width,
            equal_interaction_width(theme::DEFAULT_WINDOW_WIDTH)
        );
    }

    // @spec layout/content-chat-split Uncustomized equal width: Programmatic open rebalances to half free space
    #[test]
    fn programmatic_open_rebalances_to_half_free_space() {
        // GIVEN an uncustomized panel whose width was set for a different window
        let mut ix = InteractionState::for_window(theme::DEFAULT_WINDOW_WIDTH);
        assert!(!ix.width_customized);
        assert!(!ix.visible);
        let stale = ix.width;
        // AND content is shown (force-show path keeps equal fixed width, not fill)
        // AND current window free space allows half above min
        let current_w = 1800.0;
        assert_ne!(
            equal_interaction_width(current_w),
            stale,
            "fixture must differ from default half"
        );
        assert!(
            free_content_chat_width(current_w) / 2.0 > interaction_toggle::MIN_PANEL_WIDTH
        );
        // WHEN the panel is force-shown without a door open
        show_panel(&mut ix, current_w);
        // THEN width equals half free for the current window and stays uncustomized
        assert!(ix.visible);
        assert!(!ix.width_customized);
        assert_eq!(ix.width, equal_interaction_width(current_w));
        assert_eq!(ix.width, free_content_chat_width(current_w) / 2.0);
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
    /// User cmd-clicked a file-path reference in the terminal output.
    /// Intercepted by `main::update` (needs tabs / file-finder state).
    TerminalOpenPath {
        path: String,
        line: Option<usize>,
    },
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
///
/// `agent_input_hints` comes from global chat config.
pub fn update(
    state: &mut InteractionState,
    msg: Msg,
    highlighter: &SyntaxHighlighter,
    agent_input_hints: bool,
) -> bool {
    let mut just_opened = false;
    match msg {
        Msg::Handle(hmsg) => match hmsg {
            interaction_toggle::HandleMsg::Toggle => {
                state.visible = !state.visible;
                // Closing the panel always returns the content column.
                if !state.visible {
                    state.content_collapsed = false;
                }
                just_opened = state.visible;
            }
            interaction_toggle::HandleMsg::SetWidth(w) => {
                state.width = w;
                // First grip drag locks absolute width for the rest of the session.
                state.width_customized = true;
                // A width drag means the content column is showing again.
                state.content_collapsed = false;
            }
            interaction_toggle::HandleMsg::SetCollapsed(collapsed) => {
                state.content_collapsed = collapsed;
                // A collapsed content column needs the panel open to fill it.
                if collapsed && !state.visible {
                    state.visible = true;
                    just_opened = true;
                }
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
            handle_agent_chat(
                state,
                chat_msg,
                highlighter,
                agent_input_hints,
            );
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
        Msg::TerminalOpenPath { .. } => {
            // Intercepted by `main::update` before area dispatch.
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
    _agent_input_hints: bool,
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
            if let text_edit::EditorAction::AttachImage {
                id,
                label,
                media_type,
                bytes,
            } = action
            {
                let link = format!("[{label}](attach:{id})");
                ax.input_attachments.insert(
                    id,
                    Attachment {
                        label,
                        media_type,
                        bytes,
                    },
                );
                ax.chat_input
                    .apply_action(text_edit::EditorAction::Paste(link));
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
            // A chat message editor that fits its content but is clipped by
            // the outer chat fold can't scroll itself; it emits AutoScroll so
            // the host moves the outer scrollable. Accumulate the delta and
            // drop stick-to-bottom so a later streaming snap can't fight the
            // deliberate scroll.
            if let text_edit::EditorAction::AutoScroll { dy } = action {
                ax.pending_chat_autoscroll = Some(ax.pending_chat_autoscroll.unwrap_or(0.0) + dy);
                ax.stick_to_bottom = false;
                return;
            }
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
            agent_chat::toggle_collapse(&mut ax.chat_collapse, idx);
        }
        agent_chat::Msg::ActivateFastResponse(pick) => {
            // Fast response only — never the oneshot default-prompt list.
            // Re-check visibility so a stale click while typing is a no-op.
            let input_empty = ax.chat_input.text().trim().is_empty();
            if !crate::fast_response::visible(
                ax.session.is_streaming,
                ax.is_awaiting_user,
                input_empty,
                &ax.fast_response,
            ) {
                // no-op
            } else {
                activate_fast_response(ax, pick, highlighter);
            }
        }
        agent_chat::Msg::SendPressed => {
            let typed = ax.chat_input.text().trim().to_string();

            // Awaiting a structured choice: freeform submit is a custom answer
            // on the parked question (in-band), not cancel + next user turn.
            if let Some(plan) = plan_freeform_while_awaiting(
                ax.is_awaiting_user,
                &ax.fast_response.source,
                &typed,
            ) {
                if let Some(correlation_id) = plan.correlation_id
                    && let Some(handle) = ax.agent_handle.as_ref() {
                        handle.answer_user_choice(
                            correlation_id,
                            duckchat::UserChoiceAnswer::Custom {
                                text: plan.text,
                            },
                        );
                    }
                clear_user_choice_shell(ax);
                ax.chat_input = EditorState::new("");
                rehighlight_input(&mut ax.chat_input, highlighter);
                ax.chat_completion.visible = false;
            } else if ax.session.is_streaming {
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
                    // Next actions own empty Enter; oneshot pending does not block.
                    crate::default_prompts::next_empty_submit_text(
                        ax.session.is_streaming,
                        &ax.next_actions,
                        ax.next_action_idx,
                    )
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
        agent_chat::Msg::CycleNextAction(delta) => {
            if !ax.chat_input.text().trim().is_empty() {
                return;
            }
            if !crate::default_prompts::can_cycle_next_actions(
                ax.session.is_streaming,
                ax.next_actions.len(),
            ) {
                return;
            }
            ax.next_action_idx = crate::default_prompts::cycle_active_index(
                ax.next_actions.len(),
                ax.next_action_idx,
                delta,
            );
            // Heuristic: Tab was handled for the chat input — keep Cmd-R etc.
            // working and signal focus for the follow-up focus task.
            ax.chat_input_focused = true;
        }

        agent_chat::Msg::CancelPressed => {
            // The agent runtime will not record the still-streaming reply of
            // a cancelled turn; stash it so the next send can resync it.
            // Deltas may keep arriving until the agent stops — mark the
            // cancel so TurnComplete re-captures the grown draft.
            capture_unsynced_draft(&mut ax.session);
            ax.cancel_in_flight = true;
            if let Some(handle) = &ax.agent_handle {
                handle.cancel();
            }
            // Cancel also completes a parked choice as cancelled (handle side).
            clear_user_choice_shell(ax);
            // Drop staged follow-up so post-`TurnComplete` cannot dispatch
            // the original message after the user backed out of priming.
            clear_priming_followup(ax);
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
        agent_chat::Msg::ModelSelected(choice) => {
            // The sentinel choice is the "use project default" option, stored as
            // `None` on the session; the actual model resolves at send time. A
            // real choice carries its own harness, so a picked grok model
            // persists under the grok harness (not a hardcoded tag).
            ax.session.selected_model = choice.to_ref();
            // Persisted by `update_with_side_effects`, which has `project_root`.
            ax.model_dirty = true;
        }
        agent_chat::Msg::ChatScrolled(viewport) => {
            let bounds = viewport.bounds();
            let content = viewport.content_bounds();
            let offset_y = viewport.absolute_offset().y;
            let max_scroll = (content.height - bounds.height).max(0.0);
            let distance_from_bottom = (max_scroll - offset_y).max(0.0);
            let at_bottom = distance_from_bottom <= agent_chat::STICK_TO_BOTTOM_THRESHOLD;

            // The scrollable publishes viewport notifications for both
            // user-driven scrolls *and* content-size changes (via
            // `RedrawRequested`). To avoid racing the auto-snap task, only
            // disengage stick on a clear user scroll-up (offset decreased);
            // only engage when actually within threshold of the bottom.
            // Same-offset events caused by content growing underneath are
            // preserved.
            let prev_offset = ax.last_chat_offset_y;
            ax.last_chat_offset_y = Some(offset_y);
            let was_stuck = ax.stick_to_bottom;
            if at_bottom {
                ax.stick_to_bottom = true;
            } else if let Some(prev) = prev_offset
                && offset_y + f32::EPSILON < prev
            {
                ax.stick_to_bottom = false;
            }

            // Bottom-pin pad for fast response (when content > viewport so
            // on_scroll fires). Short content is measured via ChromeLayout.
            recompute_fast_response_top_pad(
                ax,
                bounds.height,
                content.height,
            );

            // Re-engaging stick while pure-content dirtiness was deferred
            // (user was reading history): paint the live answer now.
            if ax.stick_to_bottom
                && !was_stuck
                && ax.chat_ui_dirty
                && ax.session.is_streaming
            {
                materialize_chat_ui(ax, highlighter);
            }
        }
        agent_chat::Msg::ChromeLayout {
            viewport_h,
            content_h,
        } => {
            // Operation-based measure — works even when content fits the
            // viewport and iced suppresses on_scroll.
            recompute_fast_response_top_pad(ax, viewport_h, content_h);
        }
    }

    // When chrome is hidden (typing, streaming, empty chrome), drop the pad
    // so the next show measures from a clean baseline.
    let input_empty = ax.chat_input.text().trim().is_empty();
    if !crate::fast_response::visible(
        ax.session.is_streaming,
        ax.is_awaiting_user,
        input_empty,
        &ax.fast_response,
    ) {
        ax.fast_response_top_pad = 0.0;
    }
}

/// Recompute `fast_response_top_pad` from scroll/measure bounds. Zero when chips
/// are not visible so the next show starts clean.
fn recompute_fast_response_top_pad(
    ax: &mut AgentSession,
    viewport_h: f32,
    content_h: f32,
) {
    let input_empty = ax.chat_input.text().trim().is_empty();
    if crate::fast_response::visible(
        ax.session.is_streaming,
        ax.is_awaiting_user,
        input_empty,
        &ax.fast_response,
    ) {
        ax.fast_response_top_pad = crate::fast_response::bottom_pad(
            viewport_h,
            content_h,
            ax.fast_response_top_pad,
        );
    } else {
        ax.fast_response_top_pad = 0.0;
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
pub fn set_tentative_from_tab(ax: &mut AgentSession, editor: &EditorState, display_path: String) {
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

/// Whether a session's first turn should carry an orientation priming turn. A
/// session is primed only when it is brand-new — no resumable Claude session id
/// and no prior messages. A resumed session already carries its orientation in
/// history, so re-priming would repeat it.
fn should_prime(resumable_session_id: Option<&str>, has_prior_messages: bool) -> bool {
    resumable_session_id.is_none() && !has_prior_messages
}

/// Assemble the first-turn priming body from the available orientation parts:
/// AGENTS.md conventions (if present), the scope orientation blurb (if any),
/// and the always-present path-reference note — joined and closed with the
/// single-dot-ack instruction. All orientation rides this message body so it
/// survives the CLI's silently-dropped `--append-system-prompt` channel; the
/// path note alone keeps the body non-empty even when no `AGENTS.md` exists.
fn assemble_priming_body(agents_md: Option<&str>, scope_blurb: Option<&str>) -> String {
    let mut parts: Vec<&str> = Vec::new();
    if let Some(t) = agents_md {
        parts.push(t);
    }
    if let Some(t) = scope_blurb {
        parts.push(t);
    }
    parts.push(PATH_REFERENCE_NOTE);
    format!(
        "{}\n\nDo not respond to this message — reply with a single dot \
         (\".\") and wait for my actual instructions.",
        parts.join("\n\n"),
    )
}

/// Drop a dead agent resume id and re-dispatch the last user turn as a fresh
/// session with a history preamble.
///
/// Used when grok `session/load` returns `FS_NOT_FOUND` (cwd-key mismatch or
/// pruned session file). The user message is already on the transcript from the
/// failed attempt — this does not push a second copy. Worker-side resume state
/// is cleared so the retry opens `session/new`.
pub fn recover_from_lost_session(ax: &mut AgentSession, highlighter: &SyntaxHighlighter) {
    use crate::chat_store::{ContentBlock, Role};
    use duckchat::{ContextHook, TurnRequest};

    ax.session.agent_session_id = None;
    ax.session.session_harness = None;
    ax.priming_in_flight = false;
    // Don't fire a stashed follow-up against a half-broken resume; the retry
    // below re-sends from the transcript.
    ax.pending_followup_prompt = None;

    if let Some(handle) = ax.agent_handle.as_ref() {
        handle.clear_session_id();
    }

    // Last non-priming user message is the turn that failed mid-resume.
    let Some((last_idx, text)) = ax.session.messages.iter().enumerate().rev().find_map(|(i, m)| {
        if m.role != Role::User || m.is_priming {
            return None;
        }
        m.content.iter().find_map(|b| match b {
            ContentBlock::Text(t) if !t.is_empty() => Some((i, t.clone())),
            _ => None,
        })
    }) else {
        ax.session.is_streaming = false;
        if let Some(handle) = ax.agent_handle.as_ref()
            && let Err(e) = crate::chat_store::save_session(&ax.session, Some(handle.working_dir()))
        {
            tracing::error!("failed to persist cleared session id: {e}");
        }
        materialize_chat_ui(ax, highlighter);
        return;
    };

    let prompt = build_recovery_prompt(&mut ax.session, last_idx, &text);

    let mut system_additions = Vec::new();
    let scope = crate::scope::SessionScope {
        kind: ax.scope_kind,
        scope_key: ax.session.scope.clone(),
        change_facts: ax.scope_facts.clone(),
    };
    if let Some(out) = crate::scope::CurrentScopeHook.compute(&scope) {
        system_additions.push(out.text);
    }
    system_additions.push(PATH_REFERENCE_NOTE.to_string());

    // Idea description: re-inject on recovery so the fresh session still sees it.
    if let Some(desc) = ax.idea_description.as_ref()
        && !desc.trim().is_empty()
    {
        system_additions.push(format!("Idea description:\n\n{desc}"));
        ax.session.last_seeded_description = Some(desc.clone());
    }

    let Some(handle) = ax.agent_handle.as_ref() else {
        ax.session.is_streaming = false;
        materialize_chat_ui(ax, highlighter);
        return;
    };

    ax.session.is_streaming = true;
    ax.session.pending_text.clear();
    ax.session.pending_reasoning.clear();
    reset_answer_thrash(&mut ax.session);
    ax.cancel_in_flight = false;
    if let Err(e) = crate::chat_store::save_session(&ax.session, Some(handle.working_dir())) {
        tracing::error!("failed to persist session after resume loss: {e}");
    }
    if ax.stick_to_bottom {
        ax.pending_snap_to_bottom = true;
    }

    let mut req = TurnRequest::new(prompt, handle.working_dir().to_path_buf());
    req.system_additions = system_additions;
    req.model = Some(
        resolve_turn_model(
            ax.session.selected_model.as_ref(),
            ax.project_model_default.as_ref(),
        )
        .model,
    );
    // Attachments already went out with the failed attempt (or were empty).
    // Don't re-take from input — leave as empty for the recovery turn.
    handle.send_turn(req);

    materialize_chat_ui(ax, highlighter);
    tracing::info!(
        scope = %ax.session.scope,
        "re-dispatched last user turn after lost agent session"
    );
}

/// Send `text` as a new user turn on the active agent handle. Pushes the user
/// message into the session, marks streaming, clears the input, and rebuilds
/// the chat editor blocks. No-op if no agent handle is attached.
/// Activate a fast-response pick. Live user choice answers in-band via the
/// agent handle (no new user transcript message). Oneshot hints send the
/// option text as a normal user turn. Clears typed composer text when answering
/// a user choice so a partial custom answer is not left for a later send.
pub fn activate_fast_response(
    ax: &mut AgentSession,
    pick: crate::fast_response::FastResponsePick,
    highlighter: &SyntaxHighlighter,
) {
    use crate::fast_response::{FastResponsePick, FastResponseSource};
    use duckchat::UserChoiceAnswer;

    let source = ax.fast_response.source.clone();
    match source {
        FastResponseSource::UserChoice { correlation_id } => {
            let answer = match pick {
                FastResponsePick::Option { id } => UserChoiceAnswer::Selected { option_id: id },
            };
            // Custom freeform is handled by freeform submit path, not chip pick.
            if let Some(handle) = ax.agent_handle.as_ref() {
                handle.answer_user_choice(correlation_id, answer);
            }
            clear_user_choice_shell(ax);
            // Discard partial freeform typed while chips were still visible.
            ax.chat_input = EditorState::new("");
            rehighlight_input(&mut ax.chat_input, highlighter);
            ax.chat_completion.visible = false;
        }
        FastResponseSource::OneshotHints => {
            let FastResponsePick::Option { id: text } = pick;
            ax.agent_default_prompts.clear();
            ax.default_prompts_pending = false;
            ax.fast_response = crate::fast_response::clear();
            ax.fast_response_top_pad = 0.0;
            send_prompt_text(ax, text, highlighter);
        }
        FastResponseSource::None => {
            // No live source — nothing to activate (shell should be empty).
        }
    }
}

/// Derive the fast-response shell from settled oneshot replies when eligible.
/// Leaves a parked user-choice fill alone. Otherwise fills from oneshot or
/// clears the shell.
pub fn sync_oneshot_chips(ax: &mut AgentSession, agent_input_hints: bool) {
    use crate::fast_response::{self, FastResponseSource};

    if ax.is_awaiting_user
        || matches!(
            ax.fast_response.source,
            FastResponseSource::UserChoice { .. }
        )
    {
        return;
    }

    let prompts = ax.session_oneshot_prompts(agent_input_hints);
    if crate::default_prompts::oneshot_chips_allowed(
        ax.session.is_streaming,
        ax.is_awaiting_user,
        ax.next_actions.len(),
        agent_input_hints,
        prompts.len(),
    ) {
        ax.fast_response = fast_response::from_oneshot_hints(prompts);
    } else {
        ax.fast_response = fast_response::clear();
        ax.fast_response_top_pad = 0.0;
    }
}

/// Drop fast-response fill and awaiting flag after answer or turn end.
pub fn clear_user_choice_shell(ax: &mut AgentSession) {
    ax.fast_response = crate::fast_response::clear();
    ax.is_awaiting_user = false;
    ax.fast_response_top_pad = 0.0;
}

/// Planned freeform submit while a mid-turn choice is pending (custom answer).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FreeformWhileAwaitingPlan {
    /// Correlation id for the parked choice, when a live choice is parked.
    pub correlation_id: Option<u64>,
    /// Freeform text used as the custom answer payload.
    pub text: String,
}

/// When awaiting a user choice with non-empty freeform text, plan a custom
/// answer (in-band). Not cancel/skip and not interrupt-queue-only.
pub fn plan_freeform_while_awaiting(
    is_awaiting_user: bool,
    source: &crate::fast_response::FastResponseSource,
    typed: &str,
) -> Option<FreeformWhileAwaitingPlan> {
    let typed = typed.trim();
    if !is_awaiting_user || typed.is_empty() {
        return None;
    }
    let correlation_id = match source {
        crate::fast_response::FastResponseSource::UserChoice { correlation_id } => {
            Some(*correlation_id)
        }
        crate::fast_response::FastResponseSource::None
        | crate::fast_response::FastResponseSource::OneshotHints => None,
    };
    Some(FreeformWhileAwaitingPlan {
        correlation_id,
        text: typed.to_string(),
    })
}

/// Fill shell from a mid-turn user-choice event (options only; no cancel chip).
pub fn apply_user_choice_request(
    ax: &mut AgentSession,
    correlation_id: u64,
    prompt: Option<String>,
    options: Vec<(String, String)>,
    _allow_cancel: bool,
) {
    let _ = prompt; // reserved for future chrome (question title)
    // UI ignores allow_cancel — shell has no cancel chip; esc/freeform cancel on wire.
    ax.fast_response = crate::fast_response::from_user_choice(correlation_id, options);
    ax.is_awaiting_user = true;
}

pub fn send_prompt_text(ax: &mut AgentSession, text: String, highlighter: &SyntaxHighlighter) {
    use duckchat::{ContextHook, TurnRequest};

    // Stale agent defaults must not outlive a new turn.
    ax.clear_agent_default_prompts();

    let Some(handle) = &ax.agent_handle else {
        return;
    };

    // First-turn orientation priming.
    //
    // Claude Code's CLI silently drops `--append-system-prompt` content
    // (model can't see it via introspection, and recent CLI versions ignore
    // the flag entirely), so all orientation — AGENTS.md conventions, the
    // scope blurb, and the path-reference note — has to ride the message
    // channel. Inlining it ahead of the user's text breaks slash-command
    // parsing (`/ds-step` no longer starts the message), so we send the
    // orientation as a standalone first user turn and stash the user's actual
    // text for dispatch when that turn completes. The priming body tells the
    // model not to respond substantively (single-dot ack) so the round-trip
    // cost is minimal.
    //
    // Gated on a brand-new session (no resumable session id *and* no prior
    // messages) so legacy sessions with history but no resume id keep
    // hitting the `build_history_preamble` path below instead of getting
    // re-primed mid-conversation.
    if should_prime(ax.resumable_session_id(), !ax.session.messages.is_empty()) {
        // Resolve every orientation part. AGENTS.md is optional; the scope
        // blurb and path note are always present, so the assembled body is
        // non-empty for any fresh session — we now prime even in projects
        // without an `AGENTS.md`.
        let agents_md = crate::scope::AgentsMarkdownHook
            .compute(&handle.working_dir().to_path_buf())
            .map(|o| o.text);
        let scope = crate::scope::SessionScope {
            kind: ax.scope_kind,
            scope_key: ax.session.scope.clone(),
            change_facts: ax.scope_facts.clone(),
        };
        let scope_blurb = crate::scope::CurrentScopeHook.compute(&scope).map(|o| o.text);
        let priming_text = assemble_priming_body(agents_md.as_deref(), scope_blurb.as_deref());

        ax.session.messages.push(crate::chat_store::ChatMessage {
            role: crate::chat_store::Role::User,
            content: vec![crate::chat_store::ContentBlock::Text(priming_text.clone())],
            timestamp: String::new(),
            is_priming: true,
        });
        ax.session.is_streaming = true;
        ax.session.pending_text.clear();
        ax.session.pending_reasoning.clear();
        if ax.stick_to_bottom {
            ax.pending_snap_to_bottom = true;
        }

        ax.priming_in_flight = true;
        ax.pending_followup_prompt = Some(text);

        let mut req = TurnRequest::new(priming_text, handle.working_dir().to_path_buf());
        // All orientation now rides the message body above; `system_additions`
        // (the dropped `--append-system-prompt` channel) stays at its empty
        // default.
        // Per-chat pin wins; otherwise the project default, otherwise the
        // built-in default (grok-4.5). Prime on the same model so the resumed
        // session stays consistent.
        req.model = Some(
            resolve_turn_model(
                ax.session.selected_model.as_ref(),
                ax.project_model_default.as_ref(),
            )
            .model,
        );
        // Selection / image attachments and idea-description blurb all
        // belong to the user's intended turn — leave them on `ax` so the
        // follow-up dispatch picks them up.
        handle.send_turn(req);

        ax.chat_input = EditorState::new("");
        rehighlight_input(&mut ax.chat_input, highlighter);
        ax.chat_completion.visible = false;
        materialize_chat_ui(ax, highlighter);
        return;
    }

    // Fallback: if we have prior messages but no Claude session to `--resume`,
    // prepend the history as context so the agent isn't starting blind.
    // Happens for legacy sessions saved before session-id persistence, or if
    // the server-side session has been pruned.
    let prompt = if ax.resumable_session_id().is_none() && !ax.session.messages.is_empty() {
        build_history_preamble(&ax.session.messages) + &text
    } else {
        text.clone()
    };

    // First time Claude sees this conversation — include a scope orientation
    // blurb so the agent doesn't have to ask which change/exploration/etc.
    // we're in. Subsequent turns ride `--resume` and skip this.
    let mut system_additions = Vec::new();
    if ax.resumable_session_id().is_none() {
        let scope = crate::scope::SessionScope {
            kind: ax.scope_kind,
            scope_key: ax.session.scope.clone(),
            change_facts: ax.scope_facts.clone(),
        };
        if let Some(out) = crate::scope::CurrentScopeHook.compute(&scope) {
            system_additions.push(out.text);
        }
        system_additions.push(PATH_REFERENCE_NOTE.to_string());
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

    // Idea description: inject on the first turn (when non-empty), and
    // re-inject on any later turn where the description has changed since
    // we last told the agent. Empty descriptions are skipped — if the idea
    // later gains a description, the diff against stored `None` will trigger
    // an inject at that point.
    if let Some(desc) = ax.idea_description.as_ref()
        && !desc.trim().is_empty()
        && ax.session.last_seeded_description.as_deref() != Some(desc.as_str())
    {
        let blurb = if ax.session.last_seeded_description.is_none() {
            format!("Idea description:\n\n{desc}")
        } else {
            format!("Idea description (updated since last turn):\n\n{desc}")
        };
        system_additions.push(blurb);
        ax.session.last_seeded_description = Some(desc.clone());
    }

    // Resync a cancelled turn's kept draft: the agent runtime never recorded
    // that reply, so carry it once as agent-facing context after the user's
    // text. Clears the draft; the save below persists the clear so a resend
    // cannot replay a stale reminder. The transcript keeps only the user's
    // text.
    let prompt = apply_resync_reminder(prompt, &mut ax.session);

    ax.session.messages.push(crate::chat_store::ChatMessage {
        role: crate::chat_store::Role::User,
        content: vec![crate::chat_store::ContentBlock::Text(text)],
        timestamp: String::new(),
        is_priming: false,
    });
    ax.session.is_streaming = true;
    ax.session.pending_text.clear();
    ax.session.pending_reasoning.clear();
    reset_answer_thrash(&mut ax.session);
    // A new turn is starting — a stale cancel flag must not make this turn's
    // completed draft look cancelled at its TurnComplete.
    ax.cancel_in_flight = false;
    // Persist the transcript the moment the user turn is added, not just on
    // `TurnComplete`. Otherwise closing the app mid-turn drops the in-flight
    // message: the only prior checkpoint is the last completed turn, and on a
    // fresh session that's the synthetic priming turn — so the whole real
    // conversation is lost while the resumable `agent_session_id` survives.
    // `handle.working_dir()` is the project root the agent was spawned with,
    // which is exactly the `project_root` every other save/load site uses.
    if let Err(e) = crate::chat_store::save_session(&ax.session, Some(handle.working_dir())) {
        tracing::error!("failed to persist chat session on send: {e}");
    }
    // The user's message just grew the transcript. If they were stuck to the
    // bottom we want them to see it land there immediately — without this
    // flag the next auto-snap waits for the first `AgentEvent`.
    if ax.stick_to_bottom {
        ax.pending_snap_to_bottom = true;
    }

    let mut req = TurnRequest::new(prompt, handle.working_dir().to_path_buf());
    req.system_additions = system_additions;
    // Per-chat pin wins; otherwise the project default, otherwise the built-in
    // default (grok-4.5). On a resumed session this model overrides the
    // session's baked-in model for this turn.
    req.model = Some(
        resolve_turn_model(
            ax.session.selected_model.as_ref(),
            ax.project_model_default.as_ref(),
        )
        .model,
    );
    req.attachments = std::mem::take(&mut ax.input_attachments);
    handle.send_turn(req);

    ax.chat_input = EditorState::new("");
    rehighlight_input(&mut ax.chat_input, highlighter);
    ax.chat_completion.visible = false;
    // Drop the tentative attachment — it rode this turn but is not pinned.
    // Pinned attachments persist across messages until Cmd-R clears them.
    ax.selection_tentative = None;
    materialize_chat_ui(ax, highlighter);
}

/// Re-run markdown syntax highlighting on the chat input.
fn rehighlight_input(input: &mut EditorState, highlighter: &SyntaxHighlighter) {
    let syntax = highlighter.find_syntax("md");
    input.highlight_spans = Some(highlighter.highlight_lines(&input.lines, syntax));
}

/// The built-in default model — used when neither a per-chat pin nor a project
/// default is set. See the `harness/selection` capability.
pub(crate) fn builtin_default_model() -> ModelRef {
    ModelRef::new("grok", "grok-4.5")
}

/// Resolve the model for a turn from the three-step cascade, most specific
/// first: a per-chat `pin`, then the `project_default`, then the built-in
/// default (grok-4.5). The first level that is set wins.
pub(crate) fn resolve_turn_model(
    pin: Option<&ModelRef>,
    project_default: Option<&ModelRef>,
) -> ModelRef {
    pin.or(project_default)
        .cloned()
        .unwrap_or_else(builtin_default_model)
}

/// Render prior chat history as a text preamble for the agent. Used when we
/// don't have a Claude `--resume` session id but need to hand the agent
/// context from earlier turns. Returns a block ending with a separator; the
/// caller appends the new user message after it.
/// Build the lost-session recovery prompt: the transcript history rides the
/// prompt as a preamble, so any kept draft is already carried as a committed
/// message — clear the unsynced draft rather than adding a duplicate resync
/// reminder.
pub fn build_recovery_prompt(
    session: &mut ChatSession,
    history_end: usize,
    text: &str,
) -> String {
    session.unsynced_draft = None;
    let history = &session.messages[..history_end];
    if history.is_empty() {
        text.to_string()
    } else {
        build_history_preamble(history) + text
    }
}

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
                ContentBlock::Reasoning(t) => {
                    // Include body: agents benefit from prior thought on
                    // non-resume paths.
                    out.push_str("[Assistant reasoning]\n");
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

/// How to refresh one chat block editor during materialize.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EditorRefreshKind {
    /// Lines unchanged — keep the existing editor as-is.
    Reuse,
    /// Live answer/thinking grew by suffix only — mutate lines in place.
    InPlace { dirty_from: usize },
    /// Kind change, non-suffix edit, or new index — full `EditorState::new`.
    FullRebuild,
}

/// Decide how to refresh an editor given previous and next block lines.
fn plan_editor_refresh(old_lines: &[String], new_lines: &[String]) -> EditorRefreshKind {
    if old_lines == new_lines {
        return EditorRefreshKind::Reuse;
    }
    if let Some(dirty_from) = suffix_growth_dirty_from(old_lines, new_lines) {
        return EditorRefreshKind::InPlace { dirty_from };
    }
    EditorRefreshKind::FullRebuild
}

/// If `new_lines` is a suffix growth of `old_lines`, return the first dirty
/// line index. Shared complete-line prefix, last shared line may extend
/// (new starts with old), then optional additional lines.
fn suffix_growth_dirty_from(old: &[String], new: &[String]) -> Option<usize> {
    if new.len() < old.len() || old.is_empty() {
        return None;
    }
    let last = old.len() - 1;
    for i in 0..last {
        if old.get(i) != new.get(i) {
            return None;
        }
    }
    let old_last = &old[last];
    let new_last = new.get(last)?;
    if !new_last.starts_with(old_last.as_str()) {
        return None;
    }
    if new_last != old_last {
        Some(last)
    } else if new.len() > old.len() {
        Some(old.len())
    } else {
        // Equal content — caller should have hit Reuse via `==`.
        None
    }
}

fn make_highlighted_editor(lines: &[String], highlighter: &SyntaxHighlighter) -> EditorState {
    let content = lines.join("\n");
    let mut editor = EditorState::new(&content);
    let syntax = highlighter.find_syntax("md");
    editor.highlight_spans = Some(highlighter.highlight_lines(&editor.lines, syntax));
    editor
}

/// Tint Answer lines that fall in `write` / `next` meta-card ranges.
fn apply_meta_card_line_backgrounds(editor: &mut EditorState) {
    use crate::widget::text_edit::LineBgKind;
    let n = editor.lines.len();
    if editor.line_backgrounds.len() != n {
        editor.line_backgrounds = vec![None; n];
    } else {
        for slot in &mut editor.line_backgrounds {
            if *slot == Some(LineBgKind::MetaCard) {
                *slot = None;
            }
        }
    }
    let source = editor.lines.join("\n");
    let flags = crate::meta_card::meta_card_line_flags(&source);
    for (i, is_meta) in flags.into_iter().enumerate() {
        if is_meta && let Some(slot) = editor.line_backgrounds.get_mut(i) {
            // Prefer Match if already set (search overlay); otherwise MetaCard.
            if slot.is_none() {
                *slot = Some(LineBgKind::MetaCard);
            }
        }
    }
}

/// Update an existing editor's line buffer to `new_lines` and re-highlight.
/// Keeps editor identity (`highlight_version` bumps; not a fresh `EditorState::new`).
fn refresh_editor_in_place(
    editor: &mut EditorState,
    new_lines: &[String],
    dirty_from: usize,
    highlighter: &SyntaxHighlighter,
) {
    let lines = std::sync::Arc::make_mut(&mut editor.lines);
    lines.clear();
    lines.extend(new_lines.iter().cloned());
    if lines.is_empty() {
        lines.push(String::new());
    }

    // Markdown highlighting is stateful from line 0, so re-run the full pass
    // into the same editor. `dirty_from` documents the first changed line for
    // callers/tests; the expensive win is not constructing a new EditorState.
    let _ = dirty_from;
    let syntax = highlighter.find_syntax("md");
    editor.highlight_spans = Some(highlighter.highlight_lines(lines, syntax));
    editor.highlight_version = editor.highlight_version.wrapping_add(1);

    let last = editor.lines.len().saturating_sub(1);
    editor.cursor.line = editor.cursor.line.min(last);
    let col_max = editor.lines[editor.cursor.line].len();
    editor.cursor.col = editor.cursor.col.min(col_max);
    if let Some(a) = editor.anchor.as_mut() {
        a.line = a.line.min(last);
        let acol = editor.lines[a.line].len();
        a.col = a.col.min(acol);
    }
}

/// Rebuild the per-block chat editors for the given session.
pub fn rebuild_chat_editor(ax: &mut AgentSession, highlighter: &SyntaxHighlighter) {
    // Segment model → collapse policy → editor blocks (index-aligned).
    let segs = agent_chat::build_transcript_segments(&ax.session);
    agent_chat::sync_collapse_states(&mut ax.chat_collapse, &segs);
    let new_blocks = agent_chat::blocks_from_segments(&segs);

    let mut new_editors = Vec::with_capacity(new_blocks.len());
    for (i, block) in new_blocks.iter().enumerate() {
        if i < ax.chat_editors.len() && i < ax.chat_blocks.len() {
            let plan = plan_editor_refresh(&ax.chat_blocks[i].lines, &block.lines);
            match plan {
                EditorRefreshKind::Reuse => {
                    let existing =
                        std::mem::replace(&mut ax.chat_editors[i], EditorState::new(""));
                    new_editors.push(existing);
                }
                EditorRefreshKind::InPlace { dirty_from } => {
                    let mut existing =
                        std::mem::replace(&mut ax.chat_editors[i], EditorState::new(""));
                    refresh_editor_in_place(
                        &mut existing,
                        &block.lines,
                        dirty_from,
                        highlighter,
                    );
                    new_editors.push(existing);
                }
                EditorRefreshKind::FullRebuild => {
                    new_editors.push(make_highlighted_editor(&block.lines, highlighter));
                }
            }
        } else {
            new_editors.push(make_highlighted_editor(&block.lines, highlighter));
        }
        // Answer blocks: tint meta-card lines after lines are finalized.
        if block.kind == crate::widget::text_edit::BlockKind::Assistant
            && let Some(ed) = new_editors.last_mut()
        {
            apply_meta_card_line_backgrounds(ed);
        }
    }

    ax.chat_editors = new_editors;
    ax.chat_blocks = new_blocks;
}

/// Rebuild chat blocks/editors from `ax.session` and clear `chat_ui_dirty`.
pub fn materialize_chat_ui(ax: &mut AgentSession, highlighter: &SyntaxHighlighter) {
    rebuild_chat_editor(ax, highlighter);
    ax.chat_ui_dirty = false;
}

/// Flush pending reasoning into a committed assistant message (no-op if empty).
pub fn flush_pending_reasoning(session: &mut ChatSession) {
    if !session.pending_reasoning.is_empty() {
        let text = std::mem::take(&mut session.pending_reasoning);
        session.messages.push(ChatMessage {
            role: Role::Assistant,
            content: vec![ContentBlock::Reasoning(text)],
            timestamp: String::new(),
            is_priming: false,
        });
    }
}

/// Flush pending answer text into a committed assistant message (no-op if empty).
pub fn flush_pending_text(session: &mut ChatSession) {
    if !session.pending_text.is_empty() {
        let text = std::mem::take(&mut session.pending_text);
        session.messages.push(ChatMessage {
            role: Role::Assistant,
            content: vec![ContentBlock::Text(text)],
            timestamp: String::new(),
            is_priming: false,
        });
    }
}

/// Flush both pending reasoning and answer buffers.
pub fn flush_all_pending(session: &mut ChatSession) {
    flush_pending_reasoning(session);
    flush_pending_text(session);
}

/// Max answer-after-thought replacements allowed before thrash cancel.
/// Trip when `answer_replace_count` would exceed this (first disallowed replace).
pub const ANSWER_REPLACE_BUDGET: u32 = 1;

/// User-visible stop notice when the thrash budget trips (not a second answer).
pub const ANSWER_THRASH_STOP_NOTICE: &str =
    "Stopped: the assistant kept rewriting the same reply. Last draft kept.";

/// Reset thrash counter and trip flag (tool use, turn end, new send).
pub fn reset_answer_thrash(session: &mut ChatSession) {
    session.answer_replace_count = 0;
    session.answer_thrash_tripped = false;
}

/// Drop AGENTS.md priming flags so TurnComplete cannot dispatch a staged
/// follow-up after cancel (user CancelPressed or thrash trip).
pub fn clear_priming_followup(ax: &mut AgentSession) {
    ax.priming_in_flight = false;
    ax.pending_followup_prompt = None;
}

/// Stash the in-flight answer draft at cancellation (user cancel or thrash
/// trip) so the next send can resync it to the agent. Text already committed
/// at tool boundaries is recorded by the agent runtime and is not captured;
/// an empty draft leaves the session's unsynced draft untouched.
pub fn capture_unsynced_draft(session: &mut ChatSession) {
    if !session.pending_text.is_empty() {
        session.unsynced_draft = Some(session.pending_text.clone());
    }
}

/// Carry a cancelled turn's kept draft into the outgoing prompt as a
/// system-reminder appended **after** the user's text (front-inlining breaks
/// slash-command parsing, and `system_additions` only takes effect on a
/// session's first turn). Takes — and thus clears — the session's unsynced
/// draft, so the reminder rides exactly one send. No-op without a draft.
pub fn apply_resync_reminder(prompt: String, session: &mut ChatSession) -> String {
    let Some(draft) = session.unsynced_draft.take() else {
        return prompt;
    };
    format!(
        "{prompt}\n\n<system-reminder>\nYour previous reply was interrupted and \
         your runtime never recorded it, but the user saw it in full. Treat the \
         reply below as one you already sent — the user's message above responds \
         to it. Do not respond to this block itself.\n\n{draft}\n</system-reminder>"
    )
}

/// Commit the last draft and append the thrash stop notice. Call once when
/// the budget first trips (before cancelling the agent).
pub fn on_answer_thrash_trip(session: &mut ChatSession) {
    capture_unsynced_draft(session);
    flush_all_pending(session);
    session.messages.push(ChatMessage {
        role: Role::System,
        content: vec![ContentBlock::Text(ANSWER_THRASH_STOP_NOTICE.into())],
        timestamp: String::new(),
        is_priming: false,
    });
}

/// Apply an answer content delta to the session. Returns `true` when this
/// delta kind-switched away from pending reasoning (structural for the UI).
///
/// After reasoning, a non-empty live answer draft is **replaced** (cleared then
/// appended) so thought↔answer thrash does not concatenate or multi-commit
/// full answers. Contiguous answer deltas without a reasoning interlude still
/// append. When the thrash budget is exceeded, the session is marked tripped
/// (caller should cancel); further deltas no-op until reset.
pub fn apply_answer_content_delta(session: &mut ChatSession, text: &str) -> bool {
    if session.answer_thrash_tripped {
        return false;
    }
    let kind_switch = !session.pending_reasoning.is_empty();
    flush_pending_reasoning(session);
    if kind_switch && !session.pending_text.is_empty() {
        let next = session.answer_replace_count.saturating_add(1);
        if next > ANSWER_REPLACE_BUDGET {
            // Keep the last complete draft; do not start a truncated rewrite.
            session.answer_thrash_tripped = true;
            return true;
        }
        session.pending_text.clear();
        session.answer_replace_count = next;
    }
    session.pending_text.push_str(text);
    kind_switch
}

/// Apply a reasoning content delta to the session. Returns `true` when this
/// delta kind-switched while an answer draft is open (structural for the UI).
///
/// Does **not** commit `pending_text` — the open answer stays a live draft
/// across thought. Commit happens on tool use / turn complete via
/// [`flush_all_pending`]. No-op after thrash trip.
pub fn apply_reasoning_content_delta(session: &mut ChatSession, text: &str) -> bool {
    if session.answer_thrash_tripped {
        return false;
    }
    let kind_switch = !session.pending_text.is_empty();
    session.pending_reasoning.push_str(text);
    kind_switch
}

/// Whether the chat UI must materialize immediately for this agent event.
///
/// Pure answer/reasoning content deltas while streaming are deferred to the
/// stream UI tick unless `kind_switch` is true (answer ↔ reasoning channel
/// switch, whether or not a draft was committed). Structural events always
/// materialize.
pub fn should_materialize_chat_ui(
    evt: &crate::agent::AgentEvent,
    is_streaming: bool,
    kind_switch: bool,
) -> bool {
    use crate::agent::AgentEvent;
    if kind_switch {
        return true;
    }
    match evt {
        AgentEvent::ToolUse { .. }
        | AgentEvent::ToolResult { .. }
        | AgentEvent::TurnComplete
        | AgentEvent::Error(_)
        | AgentEvent::ProcessExited
        | AgentEvent::SessionNotFound
        | AgentEvent::UserChoiceRequest { .. } => true,
        AgentEvent::ContentDelta { .. } | AgentEvent::ReasoningDelta { .. } => !is_streaming,
        AgentEvent::Ready(_)
        | AgentEvent::CommandsAvailable(_)
        | AgentEvent::UsageUpdate { .. }
        | AgentEvent::SessionIdUpdated { .. } => false,
    }
}

/// True for events that always reshape or close the live transcript UI.
#[cfg_attr(not(test), allow(dead_code))]
pub fn is_structural_chat_event(evt: &crate::agent::AgentEvent) -> bool {
    should_materialize_chat_ui(evt, true, false)
}

/// Whether a stream UI tick should materialize pure-content dirtiness.
///
/// When the user has scrolled up to read history (`!stick_to_bottom`), skip
/// materialize so the chat column is not rebuilt at tick cadence under their
/// scroll. Session text still accumulates; the next structural event, turn
/// end, or re-stick to bottom drains the dirty flag.
pub fn should_materialize_on_stream_tick(
    is_streaming: bool,
    chat_ui_dirty: bool,
    stick_to_bottom: bool,
) -> bool {
    is_streaming && chat_ui_dirty && stick_to_bottom
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
    _agent_input_hints: bool,
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

    // Empty-input next-action cycle (only when completion is not consuming Tab).
    if *key == keyboard::Key::Named(Named::Tab)
        && ax.chat_input.text().trim().is_empty()
        && crate::default_prompts::can_cycle_next_actions(
            ax.session.is_streaming,
            ax.next_actions.len(),
        )
    {
        let delta: i8 = if mods.shift() { -1 } else { 1 };
        return AgentChatKeyResult::Dispatch(agent_chat::Msg::CycleNextAction(delta));
    }

    // Fast response hotkeys — only when chrome is visible (idle + empty input).
    // Plain Enter stays on empty-submit (list only) via TextEdit `on_submit`.
    // No ⌘⌫ cancel chip; esc / freeform-while-awaiting cancel on the wire.
    if mods.command() && !mods.shift() && !mods.alt() {
        let input_empty = ax.chat_input.text().trim().is_empty();
        // ⌘1…⌘9 → option[n]
        if let keyboard::Key::Character(c) = key
            && c.len() == 1
        {
            let ch = c.chars().next().unwrap_or('\0');
            if ch.is_ascii_digit() && ch != '0' {
                let digit = ch.to_digit(10).unwrap_or(0) as u8;
                if let Some(pick) = crate::fast_response::resolve_cmd_digit_when_visible(
                    ax.session.is_streaming,
                    ax.is_awaiting_user,
                    input_empty,
                    &ax.fast_response,
                    digit,
                ) {
                    return AgentChatKeyResult::Dispatch(agent_chat::Msg::ActivateFastResponse(
                        pick,
                    ));
                }
            }
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
#[allow(clippy::too_many_arguments)] // side-effect layer needs scope identity + project root + flags
pub fn update_with_side_effects(
    state: &mut InteractionState,
    msg: Msg,
    scope: &str,
    scope_label: &str,
    scope_kind: ScopeKind,
    project_root: Option<&std::path::Path>,
    highlighter: &SyntaxHighlighter,
    agent_input_hints: bool,
    window_w: f32,
) {
    let just_opened = update(
        state,
        msg,
        highlighter,
        agent_input_hints,
    );
    // Uncustomized panels rebalance to half free space when the door opens.
    if just_opened {
        rebalance_uncustomized(state, window_w);
    }

    // Persist a just-changed per-chat model selection. Done here (not in
    // `handle_agent_chat`) because this is the layer that has `project_root`.
    if let Some(ax) = state.active_mut()
        && ax.model_dirty
    {
        ax.model_dirty = false;
        let _ = crate::chat_store::save_session(&ax.session, project_root);
    }

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

    state.terminal_focused = state.visible && matches!(state.active_tab, ActiveTab::Terminal(_));
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
            materialize_chat_ui(&mut ax, highlighter);
            state.sessions.push(ax);
        }
        // Re-reconcile with the caller's preferred label (load_sessions_for
        // used the raw scope key, which is wrong for explorations).
        reconcile_display_names(&mut state.sessions, scope_label);
    }
    state.active_session = 0;
}

/// Persist a single session, folding any in-flight `pending_text` /
/// `pending_reasoning` into trailing assistant messages so streamed content
/// survives a crash even though it hasn't been committed to `messages` yet.
/// Reasoning is folded first (as `ContentBlock::Reasoning`), then answer text
/// (as `ContentBlock::Text`). The in-memory session is left untouched — a
/// clone is persisted. Returns whether the write succeeded.
pub fn persist_session_snapshot(session: &ChatSession, project_root: Option<&Path>) -> bool {
    let mut snapshot = session.clone();
    if !snapshot.pending_reasoning.is_empty() {
        let text = std::mem::take(&mut snapshot.pending_reasoning);
        snapshot.messages.push(ChatMessage {
            role: Role::Assistant,
            content: vec![ContentBlock::Reasoning(text)],
            timestamp: String::new(),
            is_priming: false,
        });
    }
    if !snapshot.pending_text.is_empty() {
        let text = std::mem::take(&mut snapshot.pending_text);
        snapshot.messages.push(ChatMessage {
            role: Role::Assistant,
            content: vec![ContentBlock::Text(text)],
            timestamp: String::new(),
            is_priming: false,
        });
    }
    crate::chat_store::save_session(&snapshot, project_root).is_ok()
}

/// Flush-before-mutate: persist every session an interaction holds before its
/// in-memory state is migrated, replaced, or dropped. This is the guarantee
/// that makes an in-flight turn impossible to lose to a promotion or scope
/// migration, regardless of attribution.
pub fn flush_sessions(ix: &InteractionState, project_root: Option<&Path>) {
    for ax in &ix.sessions {
        persist_session_snapshot(&ax.session, project_root);
    }
}

/// Persist the interaction's dirty sessions on the coalesced streaming flush
/// tick, clearing each dirty flag on a successful write. Bounds mid-turn loss
/// to roughly one tick interval rather than the whole send-to-turn-complete
/// window.
pub fn flush_dirty_sessions(ix: &mut InteractionState, project_root: Option<&Path>) {
    for ax in ix.sessions.iter_mut() {
        if ax.needs_flush && persist_session_snapshot(&ax.session, project_root) {
            ax.needs_flush = false;
        }
    }
}

/// Fold `incoming` sessions into `into`, never overwriting a live session.
/// Sessions whose id is already present are skipped, except on a same-id
/// collision where the copy with more messages wins. `into.instance_id` — and
/// thus its PTY/agent subscriptions — is left untouched so an in-flight stream
/// survives the merge. Sessions are re-sorted newest-first (matching load
/// order), the previously-active session is kept selected by id, and display
/// names are reconciled against `scope_label`.
pub fn merge_sessions(into: &mut InteractionState, incoming: Vec<AgentSession>, scope_label: &str) {
    let active_id = into.active().map(|ax| ax.session.id.clone());
    for inc in incoming {
        match into
            .sessions
            .iter_mut()
            .find(|s| s.session.id == inc.session.id)
        {
            Some(existing) => {
                if inc.session.messages.len() > existing.session.messages.len() {
                    *existing = inc;
                }
            }
            None => into.sessions.push(inc),
        }
    }
    into.sessions
        .sort_by_key(|s| std::cmp::Reverse(s.session.created_at_nanos));
    if let Some(id) = active_id
        && let Some(idx) = into.find_session_index(&id)
    {
        into.active_session = idx;
    }
    reconcile_display_names(&mut into.sessions, scope_label);
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
    block_highlights: Vec<(
        Vec<crate::widget::text_edit::HighlightRange>,
        Option<crate::widget::text_edit::HighlightRange>,
    )>,
    find_toolbar: Option<Element<'a, M>>,
    _agent_input_hints: bool,
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
                    crate::widget::terminal::TerminalEvent::OpenPath { path, line } => {
                        w(Msg::TerminalOpenPath { path, line })
                    }
                })
            } else {
                view_placeholder(wrap.clone())
            }
        }
        ActiveTab::Chat => {
            if let Some(ax) = state.active() {
                let model_choices = agent_chat::chat_model_choices();
                // The selector always reflects the concrete model the next turn
                // will run (pin → project default → built-in), never a "Default"
                // placeholder. The same effective model drives the meter's
                // denominator (its own context window, not a stream value).
                let effective_model = resolve_turn_model(
                    ax.session.selected_model.as_ref(),
                    ax.project_model_default.as_ref(),
                );
                let selected_model =
                    agent_chat::selected_model_choice(&model_choices, Some(&effective_model));
                let status = agent_chat::StatusInfo {
                    is_streaming: ax.session.is_streaming,
                    is_awaiting_user: ax.is_awaiting_user,
                    esc_count: ax.esc_count,
                    model_choices,
                    selected_model,
                    // Foreign/stored-but-unresumable id (e.g. harness switch).
                    // Unbound first bind and post-recovery clear stay false.
                    unresumable_stored_session: agent_chat::unresumable_stored_session(
                        ax.session.agent_session_id.is_some(),
                        ax.resumable_session_id().is_some(),
                    ),
                    context_tokens: ax.agent_input_tokens + ax.agent_output_tokens,
                    context_max: agent_chat::model_context_window(&effective_model),
                };
                let w = wrap.clone();
                let next_action_idx = crate::default_prompts::clamp_active_index(
                    ax.next_actions.len(),
                    ax.next_action_idx,
                );
                let chat_view = agent_chat::view(
                    &ax.session,
                    &ax.chat_blocks,
                    &ax.chat_editors,
                    &ax.chat_collapse,
                    &ax.chat_input,
                    ax.queue_editor.as_ref(),
                    &ax.chat_commands,
                    &ax.chat_completion,
                    status,
                    &ax.next_actions,
                    next_action_idx,
                    &ax.fast_response,
                    ax.fast_response_top_pad,
                    &ax.selection_pinned,
                    ax.selection_tentative.as_ref(),
                    block_highlights,
                )
                .map(move |m| w(Msg::AgentChat(m)));

                let session_bar = view_session_bar(state, controls, wrap.clone());
                let mut col = column![session_bar];
                if let Some(toolbar) = find_toolbar {
                    col = col.push(toolbar);
                }
                col = col.push(chat_view);
                col.height(iced::Length::Fill).into()
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

fn measure_text_with_shaping(text: &str, size: f32, shaping: iced::widget::text::Shaping) -> f32 {
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
