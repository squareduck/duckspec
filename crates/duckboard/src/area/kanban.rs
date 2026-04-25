//! Kanban area — plan future work as cards that flow through duckspec
//! lifecycle columns (Inbox → Exploring → Ready → In Progress → Completed
//! → Archived). Column for each card is derived from its attachments and
//! their on-disk state; no stored status field.

use std::collections::HashMap;
use std::path::Path;

use iced::widget::{
    Space, button, column, container, mouse_area, row, scrollable, stack, svg, text,
};
use iced::{Center, Element, Length};

const ICON_CLOSE: &[u8] = include_bytes!("../../assets/icon_close.svg");
use time::OffsetDateTime;

use super::interaction::{self, AgentSession, InteractionState, SessionControls};
use crate::chat_store::Exploration;
use crate::data::{self, ChangeData, ProjectData, StepCompletion};
use crate::highlight::SyntaxHighlighter;
use crate::kanban_store::{self, Card};
use crate::scope::ScopeKind;
use crate::theme;
use crate::widget::text_edit::{self, EditorAction, EditorState};

/// Widget id for the modal's description editor. Used by `main::update` to
/// focus the editor after `AddCard` opens a fresh card modal.
pub const DESCRIPTION_EDITOR_ID: &str = "kanban-description-editor";

// ── State ────────────────────────────────────────────────────────────────────

pub struct State {
    pub cards: Vec<Card>,
    pub selected_card: Option<String>,
    pub modal_open: bool,
    /// True when the card modal is the focused element. Flipped off when
    /// the in-modal chat or terminal takes focus so ESC-to-close doesn't
    /// fire from inside them.
    pub modal_focused: bool,
    pub description_editor: EditorState,
    /// Per-card chat + terminal state. Keyed by `Card.id` so the state
    /// survives card reopens and eventual exploration→change promotions
    /// (scope key on disk changes, card id doesn't).
    pub interactions: HashMap<String, InteractionState>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            cards: Vec::new(),
            selected_card: None,
            modal_open: false,
            modal_focused: false,
            description_editor: EditorState::new(""),
            interactions: HashMap::new(),
        }
    }
}

impl State {
    /// Load cards for the given project root. Called from `main::open_project`.
    pub fn for_project(project_root: Option<&Path>) -> Self {
        Self {
            cards: kanban_store::load(project_root),
            ..Self::default()
        }
    }

    /// Look up the card attached to a given change name, if any.
    /// Used by the Change area to offer a "jump to card" affordance.
    #[allow(dead_code)]
    pub fn card_id_for_change(&self, change_name: &str) -> Option<&str> {
        self.cards
            .iter()
            .find(|c| c.change_name.as_deref() == Some(change_name))
            .map(|c| c.id.as_str())
    }

    /// Find the id of the card whose current scope key equals `scope`.
    /// Scope matches a card's `change_name` (preferred) or `exploration_id`.
    pub fn card_id_for_scope(&self, scope: &str) -> Option<String> {
        self.cards
            .iter()
            .find(|c| {
                c.change_name.as_deref() == Some(scope)
                    || c.exploration_id.as_deref() == Some(scope)
            })
            .map(|c| c.id.clone())
    }

    /// Promote a card-owned exploration to a real change. Stamps
    /// `change_name` on the card, migrates the card's interaction state
    /// (sessions' scope, scope_kind, display names) and renames the on-disk
    /// chats directory from `chats/<exploration_id>/` to `chats/<real_name>/`.
    ///
    /// Returns the card's `InteractionState` (removed from `self.interactions`)
    /// so the caller can re-home it into `state.change.interactions` under
    /// the new change name — post-promotion, a card's chat *is* its change's
    /// chat, so the state lives on the Change side of the world. Caller is
    /// responsible for persisting `kanban.json` and inserting the returned
    /// state.
    pub fn promote_exploration(
        &mut self,
        card_id: &str,
        real_name: &str,
        project_root: Option<&Path>,
    ) -> Option<InteractionState> {
        let card = self.cards.iter_mut().find(|c| c.id == card_id)?;
        let exploration_id = card.exploration_id.clone()?;
        card.change_name = Some(real_name.to_string());

        let migrated = self.interactions.remove(card_id).map(|mut ix| {
            for ax in ix.sessions.iter_mut() {
                ax.session.scope = real_name.to_string();
                ax.scope_kind = ScopeKind::Change;
            }
            interaction::reconcile_display_names(&mut ix.sessions, real_name);
            ix
        });

        crate::chat_store::rename_scope(&exploration_id, real_name, project_root);
        migrated
    }
}

/// Current scope key for a card: `change_name` wins over `exploration_id`.
pub fn card_scope_key(card: &Card) -> Option<&str> {
    card.change_name
        .as_deref()
        .or(card.exploration_id.as_deref())
}

/// ScopeKind for the current attachment, driving the agent's current-scope hook.
pub fn card_scope_kind(card: &Card) -> Option<ScopeKind> {
    if card.change_name.is_some() {
        Some(ScopeKind::Change)
    } else if card.exploration_id.is_some() {
        Some(ScopeKind::Exploration)
    } else {
        None
    }
}

// ── Messages ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Message {
    AddCard,
    SelectCard(String),
    CloseModal,
    /// Signals whether the modal (as opposed to an embedded chat/terminal)
    /// currently holds focus. ESC-to-close respects this. The flag is
    /// normally flipped implicitly (Interaction → false, DescriptionAction
    /// and other modal-frame actions → true); this variant exists for
    /// future overrides (e.g. explicit "click backdrop" events).
    #[allow(dead_code)]
    ModalFocusChanged(bool),
    DescriptionAction(EditorAction),
    ArchiveCard(String),
    UnarchiveCard(String),
    /// Hard delete. Main loop intercepts and cascades to the attached
    /// exploration (if any) before kanban::update removes the card.
    DeleteCard(String),
    /// Spawn a new exploration for the card. Main loop intercepts to create
    /// the `Exploration` record on `state.change.explorations` with
    /// `card_id` set; kanban::update then stamps `card.exploration_id` and
    /// seeds the per-card `InteractionState`.
    StartExploration(String),
    /// Interaction message targeted at the currently selected card's
    /// chat/terminal state.
    Interaction(interaction::Msg),
    /// Navigate to the Change area and select the card's attached change.
    /// Main loop intercepts to switch areas and delegates to
    /// `area::change::Message::SelectChange`.
    OpenChange(String),
}

// ── Update ───────────────────────────────────────────────────────────────────

pub fn update(
    state: &mut State,
    message: Message,
    project: &ProjectData,
    highlighter: &SyntaxHighlighter,
    change_interactions: &mut HashMap<String, InteractionState>,
) {
    match message {
        Message::AddCard => {
            // Kanban is project-scoped on disk. Refuse creation when no
            // project is loaded so users don't end up with "phantom" cards
            // at the project-agnostic data path that disappear on reopen.
            if project.project_root.is_none() {
                tracing::warn!("kanban: ignoring AddCard with no project loaded");
                return;
            }
            let card = Card::new();
            let id = card.id.clone();
            state.cards.push(card);
            kanban_store::save(&state.cards, project.project_root.as_deref());
            open_card(state, &id, project, highlighter, change_interactions);
        }
        Message::SelectCard(id) => {
            open_card(state, &id, project, highlighter, change_interactions);
        }
        Message::CloseModal => {
            state.modal_open = false;
            state.modal_focused = false;
            // Auto-cleanup: closing a modal on a still-blank card (empty
            // description, no exploration, no change) drops the card so
            // stray Cmd-N presses don't litter the board with empty
            // placeholders. Only safe when there's nothing attached —
            // a card with an exploration/change has meaningful state
            // worth keeping even if the description is empty.
            if let Some(id) = state.selected_card.clone()
                && let Some(card) = state.cards.iter().find(|c| c.id == id)
                && card.description.trim().is_empty()
                && card.exploration_id.is_none()
                && card.change_name.is_none()
            {
                state.cards.retain(|c| c.id != id);
                state.interactions.remove(&id);
                state.selected_card = None;
                kanban_store::save(&state.cards, project.project_root.as_deref());
            }
        }
        Message::ModalFocusChanged(v) => {
            state.modal_focused = v;
        }
        Message::DescriptionAction(action) => {
            // Editing the description is a card-frame interaction, so
            // reclaim focus from any chat/terminal that stole it.
            state.modal_focused = true;
            let was_mutating = action.is_mutating();
            state.description_editor.apply_action(action);
            if was_mutating {
                if let Some(id) = state.selected_card.clone() {
                    let new_description = state.description_editor.lines.join("\n");
                    let change_name = state
                        .cards
                        .iter_mut()
                        .find(|c| c.id == id)
                        .map(|c| {
                            c.description = new_description.clone();
                            c.change_name.clone()
                        })
                        .unwrap_or(None);
                    kanban_store::save(&state.cards, project.project_root.as_deref());
                    // Mirror the latest description into the card's active
                    // chat session so the next turn re-injects it as
                    // system context. Post-promotion the state lives under
                    // `change_interactions`; pre-promotion on the kanban
                    // side.
                    let ix = match change_name.as_deref() {
                        Some(name) => change_interactions.get_mut(name),
                        None => state.interactions.get_mut(&id),
                    };
                    if let Some(ix) = ix
                        && let Some(ax) = ix.active_mut()
                    {
                        ax.card_description = Some(new_description);
                    }
                }
                let syntax = highlighter.find_syntax("md");
                state.description_editor.highlight_spans =
                    Some(highlighter.highlight_lines(&state.description_editor.lines, syntax));
            }
        }
        Message::Interaction(msg) => {
            // Routed from the chat/terminal widgets inside the modal — by
            // definition the modal is no longer the focused element for ESC.
            state.modal_focused = false;
            let Some(card_id) = state.selected_card.clone() else {
                return;
            };
            // Clone a small snapshot so we can borrow `interactions` mutably
            // without tripping the borrow checker on `cards`.
            let card_snapshot = state
                .cards
                .iter()
                .find(|c| c.id == card_id)
                .map(|c| (c.clone(), derive_title(&c.description)));
            let Some((card, derived_title)) = card_snapshot else {
                return;
            };
            let Some(scope_key) = card_scope_key(&card).map(|s| s.to_string()) else {
                return;
            };
            let scope_kind = card_scope_kind(&card).unwrap_or(ScopeKind::Exploration);
            let scope_label = if derived_title.is_empty() {
                scope_key.clone()
            } else {
                derived_title
            };
            // Post-promotion (card has a change_name) the state lives in
            // `change_interactions` keyed by change name; pre-promotion it
            // lives on the kanban side keyed by card id.
            let is_multi = card.change_name.is_some();
            let ix = match card.change_name.as_deref() {
                Some(name) => change_interactions.get_mut(name),
                None => state.interactions.get_mut(&card_id),
            };
            let Some(ix) = ix else {
                return;
            };
            match msg {
                interaction::Msg::NewSession if is_multi => {
                    interaction::ensure_sessions_with_label(
                        ix,
                        &scope_key,
                        &scope_label,
                        scope_kind,
                        project.project_root.as_deref(),
                        highlighter,
                    );
                    let new_session = AgentSession::new(scope_key.clone(), scope_kind);
                    let _ = crate::chat_store::save_session(
                        &new_session.session,
                        project.project_root.as_deref(),
                    );
                    ix.sessions.insert(0, new_session);
                    ix.active_session = 0;
                    interaction::reconcile_display_names(&mut ix.sessions, &scope_label);
                }
                interaction::Msg::SelectSession(id) if is_multi => {
                    if let Some(idx) = ix.find_session_index(&id) {
                        ix.active_session = idx;
                    }
                }
                interaction::Msg::ClearSession => {
                    interaction::clear_single_session(
                        ix,
                        &scope_key,
                        scope_kind,
                        project.project_root.as_deref(),
                    );
                }
                interaction::Msg::NewSession | interaction::Msg::SelectSession(_) => {
                    // Single-session (pre-promotion) card: ignore session
                    // management messages — no UI surfaces them anyway.
                }
                other => {
                    interaction::update_with_side_effects(
                        ix,
                        other,
                        &scope_key,
                        &scope_label,
                        scope_kind,
                        project.project_root.as_deref(),
                        highlighter,
                    );
                }
            }
        }
        Message::StartExploration(_card_id) => {
            // All mutation happens in main.rs: it owns `state.change.explorations`
            // and mints the `Exploration` with `card_id` set before calling
            // back into kanban to seed `card.exploration_id` and the
            // interaction state. See the intercept in `Message::Kanban`.
        }
        Message::OpenChange(_) => {
            // Intercepted in main.rs — switches area + selects change.
            // Dismiss the modal so kanban doesn't re-pop it on return.
            state.modal_open = false;
            state.modal_focused = false;
        }
        Message::ArchiveCard(id) => {
            state.modal_focused = true;
            let nanos = OffsetDateTime::now_local()
                .unwrap_or_else(|_| OffsetDateTime::now_utc())
                .unix_timestamp_nanos();
            if let Some(card) = state.cards.iter_mut().find(|c| c.id == id) {
                card.archived_at_nanos = Some(nanos);
            }
            kanban_store::save(&state.cards, project.project_root.as_deref());
        }
        Message::UnarchiveCard(id) => {
            state.modal_focused = true;
            if let Some(card) = state.cards.iter_mut().find(|c| c.id == id) {
                card.archived_at_nanos = None;
            }
            kanban_store::save(&state.cards, project.project_root.as_deref());
        }
        Message::DeleteCard(id) => {
            state.cards.retain(|c| c.id != id);
            state.interactions.remove(&id);
            if state.selected_card.as_deref() == Some(&id) {
                state.selected_card = None;
                state.modal_open = false;
                state.modal_focused = false;
            }
            kanban_store::save(&state.cards, project.project_root.as_deref());
        }
    }
}

fn open_card(
    state: &mut State,
    id: &str,
    project: &ProjectData,
    highlighter: &SyntaxHighlighter,
    change_interactions: &mut HashMap<String, InteractionState>,
) {
    let card_snapshot = state
        .cards
        .iter()
        .find(|c| c.id == id)
        .map(|c| (c.clone(), derive_title(&c.description)));
    if let Some((card, derived_title)) = card_snapshot {
        state.description_editor = EditorState::new(&card.description);
        let syntax = highlighter.find_syntax("md");
        state.description_editor.highlight_spans =
            Some(highlighter.highlight_lines(&state.description_editor.lines, syntax));

        // When the card has an exploration/change attached, ensure the
        // InteractionState exists and its session is materialised from
        // disk. Post-promotion the state lives in `change_interactions`
        // (the card is its change); pre-promotion on the kanban side.
        if let Some(scope_key) = card_scope_key(&card)
            && let Some(scope_kind) = card_scope_kind(&card)
        {
            let scope_key_owned = scope_key.to_string();
            let scope_label = if derived_title.is_empty() {
                scope_key_owned.clone()
            } else {
                derived_title
            };
            let ix = match card.change_name.as_deref() {
                Some(name) => change_interactions.entry(name.to_string()).or_default(),
                None => state.interactions.entry(id.to_string()).or_default(),
            };
            interaction::ensure_sessions_with_label(
                ix,
                &scope_key_owned,
                &scope_label,
                scope_kind,
                project.project_root.as_deref(),
                highlighter,
            );
            // Rehydrate the active session's ephemeral card-description
            // mirror. `send_prompt_text` reads it to decide whether to
            // inject the description as system context on the next turn.
            if let Some(ax) = ix.active_mut() {
                ax.card_description = Some(card.description.clone());
            }
            ix.visible = true;
        }
    }
    state.selected_card = Some(id.to_string());
    state.modal_open = true;
    state.modal_focused = true;
}

// ── Classifier ───────────────────────────────────────────────────────────────

/// Every possible column state for a card. `ArchivedByChange`,
/// `ArchivedManually`, and `Orphaned` all render under one visual
/// "Archived" column with a distinguishing mark. `Orphaned` lives there
/// because the card has lost its change folder — surfacing it elsewhere
/// would hide a degenerate state behind a healthy column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Column {
    Inbox,
    Exploring,
    Ready,
    InProgress,
    Completed,
    ArchivedByChange,
    ArchivedManually,
    Orphaned,
}

impl Column {
    fn is_archived(self) -> bool {
        matches!(
            self,
            Column::ArchivedByChange | Column::ArchivedManually | Column::Orphaned
        )
    }
}

/// Derive the card's current column from its attachments and their on-disk
/// state. Precedence is significant — first match wins.
pub fn classify(card: &Card, project: &ProjectData, explorations: &[Exploration]) -> Column {
    if card.archived_at_nanos.is_some() {
        return Column::ArchivedManually;
    }

    if let Some(change_name) = card.change_name.as_deref() {
        let archived = project
            .archived_changes
            .iter()
            .find(|c| data::strip_archive_prefix(&c.name) == Some(change_name));
        if archived.is_some() {
            return Column::ArchivedByChange;
        }

        let active = project.active_changes.iter().find(|c| c.name == change_name);
        let Some(active) = active else {
            return Column::Orphaned;
        };

        if !active.steps.is_empty()
            && active
                .steps
                .iter()
                .all(|s| matches!(s.completion, StepCompletion::Done))
        {
            return Column::Completed;
        }

        if has_any_artifact(active) {
            return Column::InProgress;
        }

        return Column::Ready;
    }

    let has_exploration = card
        .exploration_id
        .as_deref()
        .map(|id| explorations.iter().any(|e| e.id == id))
        .unwrap_or(false);
    if has_exploration {
        return Column::Exploring;
    }

    Column::Inbox
}

fn has_any_artifact(change: &ChangeData) -> bool {
    change.has_proposal
        || change.has_design
        || !change.cap_tree.is_empty()
        || !change.steps.is_empty()
}

/// Look up a `ChangeData` by base name, across active and archived lists.
/// Archived changes have `YYYY-MM-DD-NN-` prefixes stripped via
/// `data::strip_archive_prefix` — same convention `classify` uses.
fn find_change<'a>(project: &'a ProjectData, change_name: &str) -> Option<&'a ChangeData> {
    project
        .active_changes
        .iter()
        .find(|c| c.name == change_name)
        .or_else(|| {
            project
                .archived_changes
                .iter()
                .find(|c| data::strip_archive_prefix(&c.name) == Some(change_name))
        })
}

/// One-word summary of where the card sits in the lifecycle, driven by
/// whichever artifact is most recently present on its attached change.
/// `None` for a bare card with no exploration and no change.
fn stage_label(card: &Card, project: &ProjectData) -> Option<&'static str> {
    let change_name = match card.change_name.as_deref() {
        Some(n) => n,
        // No change yet: an exploration-only card is "Exploring"; a blank
        // card gets no stage label (the Explore CTA speaks for itself).
        None => {
            return card.exploration_id.as_ref().map(|_| "Exploring");
        }
    };
    let Some(change) = find_change(project, change_name) else {
        return Some("Change created");
    };
    if !change.steps.is_empty() {
        let all_done = change
            .steps
            .iter()
            .all(|s| matches!(s.completion, StepCompletion::Done));
        let any_progress = change.steps.iter().any(|s| {
            matches!(
                s.completion,
                StepCompletion::Partial(_, _) | StepCompletion::Done
            )
        });
        return Some(if all_done {
            "Steps applied"
        } else if any_progress {
            "Steps in progress"
        } else {
            "Steps created"
        });
    }
    if cap_tree_has_leaves(&change.cap_tree) {
        return Some("Capabilities created");
    }
    if change.has_design {
        return Some("Designed");
    }
    if change.has_proposal {
        return Some("Proposed");
    }
    Some("Change created")
}

fn cap_tree_has_leaves(nodes: &[data::TreeNode]) -> bool {
    nodes
        .iter()
        .any(|n| n.id.ends_with(".md") || cap_tree_has_leaves(&n.children))
}

/// Extract a card title from its description. Uses the first non-blank line;
/// if that line is an ATX markdown heading (up to `######` followed by
/// whitespace) the leading/trailing hashes are stripped. Returns an empty
/// string when no usable content is present — callers fall back to
/// "Untitled" for display.
pub fn derive_title(description: &str) -> String {
    let first = description
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("");
    let hash_count = first.chars().take_while(|c| *c == '#').count();
    let after_hashes = &first[hash_count..];
    let is_atx_heading = (1..=6).contains(&hash_count)
        && (after_hashes.is_empty() || after_hashes.starts_with(|c: char| c.is_whitespace()));
    if is_atx_heading {
        after_hashes.trim().trim_end_matches('#').trim().to_string()
    } else {
        first.to_string()
    }
}

// ── View ─────────────────────────────────────────────────────────────────────

/// Fixed per-column width. Columns don't flex to fill the viewport — the
/// board becomes horizontally scrollable when the window is narrower than
/// the full span of columns side-by-side. This keeps card titles readable
/// regardless of how many columns are on screen.
const KANBAN_COLUMN_WIDTH: f32 = 280.0;

const COLUMN_ORDER: [(Column, &str); 6] = [
    (Column::Inbox, "Inbox"),
    (Column::Exploring, "Exploring"),
    (Column::Ready, "Ready"),
    (Column::InProgress, "In Progress"),
    (Column::Completed, "Completed"),
    (Column::ArchivedByChange, "Archived"),
];

pub fn view<'a>(
    state: &'a State,
    project: &'a ProjectData,
    explorations: &'a [Exploration],
    change_interactions: &'a HashMap<String, InteractionState>,
) -> Element<'a, Message> {
    let mut grouped: [Vec<(&Card, Column)>; 6] = Default::default();
    for card in &state.cards {
        let col = classify(card, project, explorations);
        let idx = match col {
            Column::Inbox => 0,
            Column::Exploring => 1,
            Column::Ready => 2,
            Column::InProgress => 3,
            Column::Completed => 4,
            _ if col.is_archived() => 5,
            _ => 0,
        };
        grouped[idx].push((card, col));
    }
    for group in &mut grouped {
        group.sort_by_key(|(c, _)| -c.created_at_nanos);
    }

    let mut cols_row = row![].spacing(theme::SPACING_SM);
    for (i, (_, title)) in COLUMN_ORDER.iter().enumerate() {
        cols_row = cols_row.push(view_column(title, &grouped[i], project));
    }
    // Columns use a fixed min width so each stays readable; the board
    // becomes horizontally scrollable when the window is narrower than the
    // full span of columns side-by-side. The scrollbar reserves its own
    // gutter below the columns (rather than the default overlay) so it
    // doesn't clip the column's bottom border when visible.
    let cols_scrollbar = iced::widget::scrollable::Direction::Horizontal(
        iced::widget::scrollable::Scrollbar::new()
            .width(4)
            .scroller_width(4)
            .spacing(theme::SPACING_MD),
    );
    let cols_scroll = scrollable(cols_row)
        .direction(cols_scrollbar)
        .style(theme::thin_scrollbar)
        .width(Length::Fill)
        .height(Length::Fill);

    let has_project = project.project_root.is_some();
    let add_btn: Element<'a, Message> = if has_project {
        button(
            text("+ New card")
                .size(theme::font_md())
                .color(theme::text_primary()),
        )
        .padding([theme::SPACING_XS, theme::SPACING_MD])
        .on_press(Message::AddCard)
        .style(theme::list_item)
        .into()
    } else {
        text("Open a project to use kanban")
            .size(theme::font_sm())
            .color(theme::text_muted())
            .into()
    };

    let header = row![
        text("Kanban").size(18.0).color(theme::text_primary()),
        Space::new().width(Length::Fill),
        add_btn,
    ]
    .align_y(Center)
    .padding([theme::SPACING_SM, theme::SPACING_MD]);

    let body = container(cols_scroll)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(iced::Padding {
            top: 0.0,
            right: theme::SPACING_MD,
            bottom: theme::SPACING_MD,
            left: theme::SPACING_MD,
        });

    // The kanban board itself (header + columns).
    let board_only: Element<'a, Message> =
        container(column![header, body].height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .into();

    // Stack the modal onto the board *only* — not the whole area — so the
    // chat/terminal column on the right stays clickable while a card is
    // open.
    let board_with_modal: Element<'a, Message> = if state.modal_open
        && let Some(id) = state.selected_card.as_deref()
        && let Some(card) = state.cards.iter().find(|c| c.id == id)
    {
        let modal = view_modal(state, card, project, explorations);
        stack![board_only, modal].into()
    } else {
        board_only
    };

    // Mirror the change-area layout: content | toggle | [interaction col].
    // The interaction belongs to the currently open card — only surfaced
    // while the card modal is open, so the right column always reflects
    // the card that is currently on screen.
    // Post-promotion, a card's chat lives in `change.interactions` (the card
    // *is* its change). Pre-promotion, it lives in `state.interactions`
    // keyed by card id.
    let ix = if state.modal_open
        && let Some(id) = state.selected_card.as_deref()
        && let Some(card) = state.cards.iter().find(|c| c.id == id)
    {
        match card.change_name.as_deref() {
            Some(name) => change_interactions.get(name),
            None => state.interactions.get(id),
        }
    } else {
        None
    };

    // A card "becomes" its change once promoted: surface the same multi-
    // session UI (CHATS header, session selector, "+") the Change area uses.
    // Pre-promotion (exploration only) we keep Single mode — explorations
    // are single-session by design.
    let controls = state
        .selected_card
        .as_deref()
        .and_then(|id| state.cards.iter().find(|c| c.id == id))
        .and_then(|c| c.change_name.as_deref())
        .map(|_| SessionControls::Multi)
        .unwrap_or(SessionControls::Single);

    let visible = ix.is_some_and(|i| i.visible);
    let width = ix.map_or(theme::INTERACTION_COLUMN_WIDTH, |i| i.width);

    let mut main_row = row![
        container(board_with_modal)
            .width(Length::Fill)
            .height(Length::Fill),
    ];

    // Toggle handle is only shown when there's actually an interaction to
    // toggle — i.e. the selected card has an exploration or change attached.
    if ix.is_some() {
        let toggle = crate::widget::interaction_toggle::view(visible, width, |m| {
            Message::Interaction(interaction::Msg::Handle(m))
        });
        main_row = main_row.push(toggle);
    }

    if let Some(ix) = ix
        && ix.visible
    {
        let ix_col = interaction::view_column(ix, Message::Interaction, controls);
        main_row = main_row.push(
            container(ix_col)
                .width(ix.width)
                .height(Length::Fill)
                .style(theme::surface),
        );
    }

    main_row.height(Length::Fill).into()
}

fn view_column<'a>(
    title: &'a str,
    cards: &[(&'a Card, Column)],
    project: &'a ProjectData,
) -> Element<'a, Message> {
    let count = cards.len();
    let header = row![
        text(title)
            .size(theme::font_md())
            .color(theme::text_primary()),
        Space::new().width(Length::Fill),
        text(count.to_string())
            .size(theme::font_sm())
            .color(theme::text_muted()),
    ]
    .align_y(Center)
    .padding([theme::SPACING_SM, theme::SPACING_MD]);

    let body: Element<'a, Message> = if cards.is_empty() {
        container(
            text("No cards")
                .size(theme::font_sm())
                .color(theme::text_muted()),
        )
        .padding(theme::SPACING_MD)
        .into()
    } else {
        let mut card_col = column![].spacing(theme::SPACING_SM);
        for (card, col) in cards {
            card_col = card_col.push(view_card_row(card, *col, project));
        }
        scrollable(container(card_col).padding(theme::SPACING_MD))
            .direction(theme::thin_scrollbar_direction())
            .style(theme::thin_scrollbar)
            .height(Length::Fill)
            .into()
    };

    let inner = column![
        header,
        container(Space::new().height(1.0).width(Length::Fill)).style(column_divider),
        body,
    ]
    .height(Length::Fill);

    container(inner)
        .width(Length::Fixed(KANBAN_COLUMN_WIDTH))
        .height(Length::Fill)
        .style(column_style)
        .into()
}

fn view_card_row<'a>(
    card: &'a Card,
    col: Column,
    project: &'a ProjectData,
) -> Element<'a, Message> {
    let derived = derive_title(&card.description);
    let title = if derived.is_empty() {
        "Untitled".to_string()
    } else {
        derived
    };
    let mut inner = column![text(title).size(theme::font_md()).color(theme::text_primary())]
        .spacing(theme::SPACING_XS);

    // Subtle subtitle — archived mark takes priority; otherwise step
    // progress for columns where a change is attached.
    let subtitle = archived_mark(col)
        .map(str::to_string)
        .or_else(|| step_progress_subtitle(card, col, project));
    if let Some(sub) = subtitle {
        inner = inner.push(text(sub).size(theme::font_sm()).color(theme::text_muted()));
    }

    button(container(inner).padding(theme::SPACING_SM).width(Length::Fill))
        .on_press(Message::SelectCard(card.id.clone()))
        .style(card_button)
        .width(Length::Fill)
        .into()
}

fn step_progress_subtitle(
    card: &Card,
    col: Column,
    project: &ProjectData,
) -> Option<String> {
    if !matches!(col, Column::InProgress | Column::Completed) {
        return None;
    }
    let change = find_change(project, card.change_name.as_deref()?)?;
    if change.steps.is_empty() {
        return None;
    }
    let total = change.steps.len();
    let done = change
        .steps
        .iter()
        .filter(|s| matches!(s.completion, StepCompletion::Done))
        .count();
    Some(format!("{done}/{total} steps"))
}

fn archived_mark(col: Column) -> Option<&'static str> {
    match col {
        Column::ArchivedByChange => Some("via change"),
        Column::ArchivedManually => Some("manual"),
        Column::Orphaned => Some("orphaned"),
        _ => None,
    }
}

// ── Modal ────────────────────────────────────────────────────────────────────

fn view_modal<'a>(
    state: &'a State,
    card: &'a Card,
    project: &'a ProjectData,
    explorations: &'a [Exploration],
) -> Element<'a, Message> {
    let col = classify(card, project, explorations);
    let metadata = view_modal_metadata(state, card, col, project);

    // Full-screen clickable backdrop — pressing anywhere closes the modal.
    let backdrop = mouse_area(
        container(Space::new().width(Length::Fill).height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(modal_backdrop),
    )
    .on_press(Message::CloseModal);

    // The panel itself absorbs presses so clicks inside don't bubble to the
    // backdrop and close the modal. Reusing `ModalFocusChanged(true)` as a
    // semantically correct focus-claim on any in-panel click.
    let panel = mouse_area(
        container(metadata)
            .max_width(640.0)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(modal_panel),
    )
    .on_press(Message::ModalFocusChanged(true));

    let centered_panel = container(panel)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding([48.0, 32.0])
        .center_x(Length::Fill);

    stack![backdrop, centered_panel].into()
}

fn view_modal_metadata<'a>(
    state: &'a State,
    card: &'a Card,
    col: Column,
    project: &'a ProjectData,
) -> Element<'a, Message> {
    let derived = derive_title(&card.description);
    let title_label = if derived.is_empty() {
        "Untitled".to_string()
    } else {
        derived
    };
    let title_view = text(title_label).size(22.0).color(theme::text_primary());

    let close_icon = svg(svg::Handle::from_memory(ICON_CLOSE))
        .width(16.0)
        .height(16.0)
        .style(theme::svg_tint(theme::text_secondary()));
    let close_btn = button(close_icon)
        .on_press(Message::CloseModal)
        .padding(theme::SPACING_XS)
        .style(theme::list_item);

    let title_row = row![
        title_view,
        Space::new().width(Length::Fill),
        close_btn,
    ]
    .align_y(Center)
    .spacing(theme::SPACING_SM);

    // Status row — left: stage indicator (derived from the latest artifact
    // present on the attached change); right: one navigation/CTA button.
    // Artifacts themselves live in the Change area; this row just tells
    // the user where the card is in the lifecycle and gives them a single
    // jump point.
    let stage = stage_label(card, project);
    let action: Option<Element<'a, Message>> =
        if let Some(change_name) = card.change_name.as_deref() {
            Some(
                button(
                    text(format!("Change - {change_name}"))
                        .size(theme::font_sm())
                        .color(theme::text_primary()),
                )
                .on_press(Message::OpenChange(change_name.to_string()))
                .padding([theme::SPACING_XS, theme::SPACING_MD])
                .style(pill)
                .into(),
            )
        } else if card.exploration_id.is_none() && !col.is_archived() {
            Some(
                button(
                    text("Explore")
                        .size(theme::font_sm())
                        .color(theme::accent()),
                )
                .on_press(Message::StartExploration(card.id.clone()))
                .padding([theme::SPACING_XS, theme::SPACING_MD])
                .style(pill_accent)
                .into(),
            )
        } else {
            None
        };
    let status_row: Option<Element<'a, Message>> = match (stage, action) {
        (None, None) => None,
        (stage, action) => {
            let stage_el: Element<'a, Message> = match stage {
                Some(label) => text(label)
                    .size(theme::font_sm())
                    .color(theme::text_secondary())
                    .into(),
                None => Space::new().width(0.0).height(0.0).into(),
            };
            let mut r = row![stage_el, Space::new().width(Length::Fill)]
                .align_y(Center)
                .spacing(theme::SPACING_SM);
            if let Some(action) = action {
                r = r.push(action);
            }
            Some(r.into())
        }
    };

    let description_editor = text_edit::TextEdit::new(
        &state.description_editor,
        Message::DescriptionAction,
    )
    .id(DESCRIPTION_EDITOR_ID)
    .show_gutter(false)
    .word_wrap(true)
    .placeholder("Description (markdown)")
    .transparent_bg(true);

    let description_box = container(description_editor)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(theme::SPACING_SM)
        .style(modal_editor_wrap);

    let mut body = column![title_row].spacing(theme::SPACING_MD);
    if let Some(status_row) = status_row {
        body = body.push(status_row);
    }
    body = body.push(description_box);
    body = body.push(view_action_row(card, col));

    container(body)
        .padding(theme::SPACING_XL)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn view_action_row<'a>(card: &'a Card, col: Column) -> Element<'a, Message> {
    let lifecycle: Element<'a, Message> = if col.is_archived() {
        button(
            text("Unarchive")
                .size(theme::font_md())
                .color(theme::text_primary()),
        )
        .on_press(Message::UnarchiveCard(card.id.clone()))
        .padding([theme::SPACING_SM, theme::SPACING_LG])
        .style(action_btn)
        .into()
    } else {
        button(
            text("Archive")
                .size(theme::font_md())
                .color(theme::text_primary()),
        )
        .on_press(Message::ArchiveCard(card.id.clone()))
        .padding([theme::SPACING_SM, theme::SPACING_LG])
        .style(action_btn)
        .into()
    };

    let delete_btn = button(
        text("Delete")
            .size(theme::font_md())
            .color(theme::error()),
    )
    .on_press(Message::DeleteCard(card.id.clone()))
    .padding([theme::SPACING_SM, theme::SPACING_LG])
    .style(action_btn);

    row![Space::new().width(Length::Fill), lifecycle, delete_btn]
        .spacing(theme::SPACING_SM)
        .align_y(Center)
        .into()
}

// ── Breadcrumbs ──────────────────────────────────────────────────────────────

pub fn breadcrumbs() -> Vec<String> {
    vec!["Kanban".into()]
}

// ── Styles ───────────────────────────────────────────────────────────────────

fn column_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(theme::bg_surface().into()),
        border: iced::Border {
            color: theme::border_color(),
            width: 1.0,
            radius: 6.0.into(),
        },
        ..Default::default()
    }
}

fn column_divider(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(theme::border_color().into()),
        ..Default::default()
    }
}

fn card_button(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: Some(theme::bg_base().into()),
        text_color: theme::text_primary(),
        border: iced::Border {
            color: theme::border_color(),
            width: 1.0,
            radius: 4.0.into(),
        },
        ..Default::default()
    };
    match status {
        button::Status::Hovered | button::Status::Pressed => button::Style {
            border: iced::Border {
                color: theme::accent(),
                ..base.border
            },
            ..base
        },
        _ => base,
    }
}

fn modal_backdrop(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(
            iced::Color {
                a: 0.55,
                ..iced::Color::BLACK
            }
            .into(),
        ),
        ..Default::default()
    }
}

fn modal_panel(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(theme::bg_surface().into()),
        border: iced::Border {
            color: theme::border_color(),
            width: 1.0,
            radius: 12.0.into(),
        },
        shadow: iced::Shadow {
            color: iced::Color {
                a: 0.35,
                ..iced::Color::BLACK
            },
            offset: iced::Vector::new(0.0, 12.0),
            blur_radius: 32.0,
        },
        ..Default::default()
    }
}

fn modal_editor_wrap(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(theme::bg_base().into()),
        border: iced::Border {
            color: theme::border_color(),
            width: 1.0,
            radius: 6.0.into(),
        },
        ..Default::default()
    }
}

fn action_btn(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => theme::bg_hover(),
        _ => theme::bg_elevated(),
    };
    button::Style {
        background: Some(bg.into()),
        text_color: theme::text_primary(),
        border: iced::Border {
            color: theme::border_color(),
            width: 1.0,
            radius: 6.0.into(),
        },
        ..Default::default()
    }
}

fn pill(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => theme::bg_hover(),
        _ => theme::bg_elevated(),
    };
    button::Style {
        background: Some(bg.into()),
        text_color: theme::text_primary(),
        border: iced::Border {
            color: theme::border_color(),
            width: 1.0,
            radius: 999.0.into(),
        },
        ..Default::default()
    }
}

fn pill_accent(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => theme::bg_hover(),
        _ => theme::bg_elevated(),
    };
    button::Style {
        background: Some(bg.into()),
        text_color: theme::accent(),
        border: iced::Border {
            color: theme::accent(),
            width: 1.0,
            radius: 999.0.into(),
        },
        ..Default::default()
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{StepInfo, TreeNode};

    fn card() -> Card {
        Card::new()
    }
    fn exp(id: &str) -> Exploration {
        Exploration {
            id: id.into(),
            display_name: id.into(),
            card_id: None,
        }
    }
    fn change(name: &str) -> ChangeData {
        ChangeData {
            name: name.into(),
            prefix: String::new(),
            has_proposal: false,
            has_design: false,
            cap_tree: vec![],
            steps: vec![],
        }
    }
    fn step(done: bool) -> StepInfo {
        StepInfo {
            id: "s".into(),
            label: "s".into(),
            completion: if done {
                StepCompletion::Done
            } else {
                StepCompletion::Partial(0, 1)
            },
        }
    }
    fn project(active: Vec<ChangeData>, archived: Vec<ChangeData>) -> ProjectData {
        let mut p = ProjectData::default();
        p.active_changes = active;
        p.archived_changes = archived;
        p
    }

    #[test]
    fn inbox_when_no_attachments() {
        let c = card();
        let p = project(vec![], vec![]);
        assert_eq!(classify(&c, &p, &[]), Column::Inbox);
    }

    #[test]
    fn exploring_when_exploration_exists() {
        let mut c = card();
        c.exploration_id = Some("e1".into());
        let p = project(vec![], vec![]);
        assert_eq!(classify(&c, &p, &[exp("e1")]), Column::Exploring);
    }

    #[test]
    fn inbox_when_exploration_id_dangles() {
        let mut c = card();
        c.exploration_id = Some("missing".into());
        let p = project(vec![], vec![]);
        assert_eq!(classify(&c, &p, &[]), Column::Inbox);
    }

    #[test]
    fn ready_when_change_exists_with_no_artifacts() {
        let mut c = card();
        c.change_name = Some("ch".into());
        let p = project(vec![change("ch")], vec![]);
        assert_eq!(classify(&c, &p, &[]), Column::Ready);
    }

    #[test]
    fn in_progress_when_change_has_artifact() {
        let mut c = card();
        c.change_name = Some("ch".into());
        let mut ch = change("ch");
        ch.has_proposal = true;
        let p = project(vec![ch], vec![]);
        assert_eq!(classify(&c, &p, &[]), Column::InProgress);
    }

    #[test]
    fn in_progress_when_cap_tree_nonempty() {
        let mut c = card();
        c.change_name = Some("ch".into());
        let mut ch = change("ch");
        ch.cap_tree = vec![TreeNode {
            id: "x".into(),
            label: "x".into(),
            children: vec![],
        }];
        let p = project(vec![ch], vec![]);
        assert_eq!(classify(&c, &p, &[]), Column::InProgress);
    }

    #[test]
    fn completed_when_all_steps_done() {
        let mut c = card();
        c.change_name = Some("ch".into());
        let mut ch = change("ch");
        ch.steps = vec![step(true), step(true)];
        let p = project(vec![ch], vec![]);
        assert_eq!(classify(&c, &p, &[]), Column::Completed);
    }

    #[test]
    fn in_progress_when_steps_partial() {
        let mut c = card();
        c.change_name = Some("ch".into());
        let mut ch = change("ch");
        ch.steps = vec![step(true), step(false)];
        let p = project(vec![ch], vec![]);
        assert_eq!(classify(&c, &p, &[]), Column::InProgress);
    }

    #[test]
    fn archived_by_change_when_change_archived() {
        let mut c = card();
        c.change_name = Some("ch".into());
        let mut archived = change("2026-04-24-01-ch");
        archived.prefix = "2026-04-24-01-".into();
        let p = project(vec![], vec![archived]);
        assert_eq!(classify(&c, &p, &[]), Column::ArchivedByChange);
    }

    #[test]
    fn orphaned_when_change_missing() {
        let mut c = card();
        c.change_name = Some("gone".into());
        let p = project(vec![], vec![]);
        assert_eq!(classify(&c, &p, &[]), Column::Orphaned);
    }

    #[test]
    fn derive_title_empty() {
        assert_eq!(derive_title(""), "");
    }

    #[test]
    fn derive_title_plain_line() {
        assert_eq!(derive_title("hello world"), "hello world");
    }

    #[test]
    fn derive_title_skips_blank_leading_lines() {
        assert_eq!(derive_title("\n\n  hi\nsecond"), "hi");
    }

    #[test]
    fn derive_title_atx_heading() {
        assert_eq!(derive_title("# Heading"), "Heading");
        assert_eq!(derive_title("### Third level"), "Third level");
        assert_eq!(derive_title("###### Six"), "Six");
    }

    #[test]
    fn derive_title_strips_trailing_hashes() {
        assert_eq!(derive_title("## Heading ##"), "Heading");
    }

    #[test]
    fn derive_title_seven_hashes_is_not_heading() {
        assert_eq!(derive_title("####### nope"), "####### nope");
    }

    #[test]
    fn derive_title_hash_no_space_is_not_heading() {
        assert_eq!(derive_title("#no-space"), "#no-space");
    }

    #[test]
    fn archived_manually_takes_priority() {
        let mut c = card();
        c.archived_at_nanos = Some(1);
        c.exploration_id = Some("e1".into());
        c.change_name = Some("ch".into());
        let p = project(vec![change("ch")], vec![]);
        assert_eq!(classify(&c, &p, &[exp("e1")]), Column::ArchivedManually);
    }
}
