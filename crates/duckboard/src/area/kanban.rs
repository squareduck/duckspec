//! Kanban area — plan future work as cards that flow through duckspec
//! lifecycle columns (Inbox → Exploring → Ready → In Progress → Completed
//! → Archived). Column for each card is derived from its attachments and
//! their on-disk state; no stored status field.

use std::collections::HashMap;
use std::path::Path;

use iced::widget::{Space, button, column, container, row, scrollable, stack, text};
use iced::{Center, Element, Fill, Length};
use time::OffsetDateTime;

use super::interaction::{self, InteractionState, SessionControls};
use crate::chat_store::Exploration;
use crate::data::{self, ChangeData, ProjectData, StepCompletion};
use crate::highlight::SyntaxHighlighter;
use crate::kanban_store::{self, Card};
use crate::scope::ScopeKind;
use crate::theme;
use crate::widget::text_edit::{self, EditorAction, EditorState};

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
    /// Caller is responsible for persisting `kanban.json`.
    pub fn promote_exploration(
        &mut self,
        card_id: &str,
        real_name: &str,
        project_root: Option<&Path>,
    ) {
        let Some(card) = self.cards.iter_mut().find(|c| c.id == card_id) else {
            return;
        };
        let Some(exploration_id) = card.exploration_id.clone() else {
            return;
        };
        card.change_name = Some(real_name.to_string());

        if let Some(ix) = self.interactions.get_mut(card_id) {
            for ax in ix.sessions.iter_mut() {
                ax.session.scope = real_name.to_string();
                ax.scope_kind = ScopeKind::Change;
            }
            interaction::reconcile_display_names(&mut ix.sessions, real_name);
        }

        crate::chat_store::rename_scope(&exploration_id, real_name, project_root);
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
    DiscardCard(String),
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
    /// Navigate to the Change area and open an artifact of the card's
    /// attached change. Main loop intercepts to switch areas and delegates
    /// to `area::change::Message::OpenArtifact`.
    OpenArtifact {
        change: String,
        artifact_id: String,
    },
}

// ── Update ───────────────────────────────────────────────────────────────────

pub fn update(
    state: &mut State,
    message: Message,
    project: &ProjectData,
    highlighter: &SyntaxHighlighter,
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
            open_card(state, &id, project, highlighter);
        }
        Message::SelectCard(id) => {
            open_card(state, &id, project, highlighter);
        }
        Message::CloseModal => {
            state.modal_open = false;
            state.modal_focused = false;
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
                if let Some(id) = state.selected_card.clone()
                    && let Some(card) = state.cards.iter_mut().find(|c| c.id == id)
                {
                    card.description = state.description_editor.lines.join("\n");
                    kanban_store::save(&state.cards, project.project_root.as_deref());
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
            let Some(ix) = state.interactions.get_mut(&card_id) else {
                return;
            };
            match msg {
                interaction::Msg::ClearSession => {
                    interaction::clear_single_session(
                        ix,
                        &scope_key,
                        scope_kind,
                        project.project_root.as_deref(),
                    );
                }
                interaction::Msg::NewSession | interaction::Msg::SelectSession(_) => {
                    // Cards are single-session; ignore.
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
        Message::OpenArtifact { .. } => {
            // Intercepted in main.rs — switches to the Change area and
            // routes to `area::change::Message::OpenArtifact`. Dismiss the
            // modal here so returning to Kanban later doesn't pop it open
            // over the kanban board.
            state.modal_open = false;
            state.modal_focused = false;
        }
        Message::DiscardCard(id) => {
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
        // per-card InteractionState exists and its session is materialised
        // from disk (same path change::SelectChange uses).
        if let Some(scope_key) = card_scope_key(&card)
            && let Some(scope_kind) = card_scope_kind(&card)
        {
            let scope_key_owned = scope_key.to_string();
            let scope_label = if derived_title.is_empty() {
                scope_key_owned.clone()
            } else {
                derived_title
            };
            let ix = state.interactions.entry(id.to_string()).or_default();
            interaction::ensure_sessions_with_label(
                ix,
                &scope_key_owned,
                &scope_label,
                scope_kind,
                project.project_root.as_deref(),
                highlighter,
            );
            ix.visible = true;
        }
    }
    state.selected_card = Some(id.to_string());
    state.modal_open = true;
    state.modal_focused = true;
}

// ── Classifier ───────────────────────────────────────────────────────────────

/// Every possible column state for a card. The three `Archived*` variants
/// all render under one visual "Archived" column with a distinguishing mark.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Column {
    Inbox,
    Exploring,
    Ready,
    InProgress,
    Completed,
    ArchivedDone,
    ArchivedAbandoned,
    ArchivedDiscarded,
}

impl Column {
    fn is_archived(self) -> bool {
        matches!(
            self,
            Column::ArchivedDone | Column::ArchivedAbandoned | Column::ArchivedDiscarded
        )
    }
}

/// Derive the card's current column from its attachments and their on-disk
/// state. Precedence is significant — first match wins.
pub fn classify(card: &Card, project: &ProjectData, explorations: &[Exploration]) -> Column {
    if card.archived_at_nanos.is_some() {
        return Column::ArchivedDiscarded;
    }

    if let Some(change_name) = card.change_name.as_deref() {
        let archived = project
            .archived_changes
            .iter()
            .find(|c| data::strip_archive_prefix(&c.name) == Some(change_name));
        if archived.is_some() {
            return Column::ArchivedDone;
        }

        let active = project.active_changes.iter().find(|c| c.name == change_name);
        let Some(active) = active else {
            return Column::ArchivedAbandoned;
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

/// Build `(label, artifact_id)` pairs for every opened file on a change.
/// Entries match the IDs `area::change::open_artifact` understands: the
/// on-disk relative path inside `duckspec/`. Proposal and design first,
/// then cap-tree leaves, then steps.
fn artifact_entries(change: &ChangeData) -> Vec<(String, String)> {
    let mut out = Vec::new();
    if change.has_proposal {
        out.push(("proposal.md".into(), format!("{}/proposal.md", change.prefix)));
    }
    if change.has_design {
        out.push(("design.md".into(), format!("{}/design.md", change.prefix)));
    }
    collect_tree_leaves(&change.cap_tree, &mut out);
    for step in &change.steps {
        out.push((step.label.clone(), step.id.clone()));
    }
    out
}

/// Walk the cap-tree and emit leaf artifacts (anything whose `id` ends in
/// `.md`). Intermediate nodes are capability-group headers and don't open.
fn collect_tree_leaves(nodes: &[data::TreeNode], out: &mut Vec<(String, String)>) {
    for node in nodes {
        if node.id.ends_with(".md") {
            out.push((node.label.clone(), node.id.clone()));
        }
        if !node.children.is_empty() {
            collect_tree_leaves(&node.children, out);
        }
    }
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

const COLUMN_ORDER: [(Column, &str); 6] = [
    (Column::Inbox, "Inbox"),
    (Column::Exploring, "Exploring"),
    (Column::Ready, "Ready"),
    (Column::InProgress, "In Progress"),
    (Column::Completed, "Completed"),
    (Column::ArchivedDone, "Archived"),
];

pub fn view<'a>(
    state: &'a State,
    project: &'a ProjectData,
    explorations: &'a [Exploration],
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

    let body = container(cols_row)
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
    // The interaction belongs to the currently selected card — surfaced
    // at area level so it stays usable while/after the modal is open.
    let ix = state
        .selected_card
        .as_deref()
        .and_then(|id| state.interactions.get(id));

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
        let ix_col = interaction::view_column(ix, Message::Interaction, SessionControls::Single);
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
    .padding([theme::SPACING_XS, theme::SPACING_SM]);

    let body: Element<'a, Message> = if cards.is_empty() {
        container(
            text("No cards")
                .size(theme::font_sm())
                .color(theme::text_muted()),
        )
        .padding(theme::SPACING_SM)
        .into()
    } else {
        let mut card_col = column![].spacing(theme::SPACING_XS);
        for (card, col) in cards {
            card_col = card_col.push(view_card_row(card, *col, project));
        }
        scrollable(container(card_col).padding(theme::SPACING_XS))
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
        .width(Fill)
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
        Column::ArchivedDone => Some("done"),
        Column::ArchivedAbandoned => Some("abandoned"),
        Column::ArchivedDiscarded => Some("discarded"),
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
    let metadata = view_modal_metadata(state, card, col, project, explorations);

    let panel = container(
        container(metadata)
            .max_width(560.0)
            .width(Length::Fill),
    )
    .max_width(560.0)
    .style(modal_panel);

    container(column![Space::new().height(80.0), panel].align_x(Center))
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Center)
        .style(modal_backdrop)
        .into()
}

fn view_modal_metadata<'a>(
    state: &'a State,
    card: &'a Card,
    col: Column,
    project: &'a ProjectData,
    explorations: &'a [Exploration],
) -> Element<'a, Message> {
    let col_label = column_label(col);

    let derived = derive_title(&card.description);
    let title_label = if derived.is_empty() {
        "Untitled".to_string()
    } else {
        derived
    };
    let title_view = text(title_label).size(18.0).color(theme::text_primary());

    let delete_btn = button(
        text("Delete")
            .size(theme::font_md())
            .color(theme::error()),
    )
    .on_press(Message::DeleteCard(card.id.clone()))
    .padding([theme::SPACING_XS, theme::SPACING_MD])
    .style(theme::list_item);

    let title_row = row![
        title_view,
        Space::new().width(Length::Fill),
        delete_btn,
    ]
    .align_y(Center)
    .spacing(theme::SPACING_SM);

    let state_row = row![
        text("State:")
            .size(theme::font_sm())
            .color(theme::text_secondary()),
        text(col_label)
            .size(theme::font_sm())
            .color(theme::text_primary()),
    ]
    .spacing(theme::SPACING_SM);

    let description_editor = text_edit::TextEdit::new(
        &state.description_editor,
        Message::DescriptionAction,
    )
    .show_gutter(false)
    .word_wrap(true)
    .placeholder("Description (markdown)")
    .transparent_bg(true);

    let description_box = container(description_editor)
        .width(Length::Fill)
        .height(180.0)
        .padding(theme::SPACING_SM)
        .style(modal_editor_wrap);

    let exploration_line = match card.exploration_id.as_deref() {
        Some(id) => {
            let name = explorations
                .iter()
                .find(|e| e.id == id)
                .map(|e| e.display_name.as_str())
                .unwrap_or("(missing)");
            format!("Exploration: {name}")
        }
        None => "Exploration: (none)".into(),
    };
    let change_line = match card.change_name.as_deref() {
        Some(name) => format!("Change: {name}"),
        None => "Change: (none)".into(),
    };

    let mut body = column![
        title_row,
        state_row,
        container(Space::new().height(1.0).width(Length::Fill)).style(column_divider),
        description_box,
    ]
    .spacing(theme::SPACING_SM);

    // "Start exploration" — only surfaced when the card is truly bare;
    // once attachments exist the interaction column on the right takes over.
    if card_scope_key(card).is_none() && !col.is_archived() {
        let start_btn = button(
            text("Start exploration")
                .size(theme::font_md())
                .color(theme::accent()),
        )
        .on_press(Message::StartExploration(card.id.clone()))
        .padding([theme::SPACING_XS, theme::SPACING_MD])
        .style(theme::list_item);
        body = body.push(Space::new().height(theme::SPACING_XS));
        body = body.push(start_btn);
    }

    // Artifact list — only when an attached change has something to open.
    if let Some(change_name) = card.change_name.as_deref()
        && let Some(change) = find_change(project, change_name)
    {
        let artifacts = artifact_entries(change);
        if !artifacts.is_empty() {
            body = body.push(
                container(Space::new().height(1.0).width(Length::Fill)).style(column_divider),
            );
            body = body.push(
                text("Artifacts")
                    .size(theme::font_sm())
                    .color(theme::text_secondary()),
            );
            let mut list = column![].spacing(theme::SPACING_XS);
            for (label, artifact_id) in artifacts {
                let btn = button(
                    text(label)
                        .size(theme::font_md())
                        .color(theme::text_primary()),
                )
                .on_press(Message::OpenArtifact {
                    change: change_name.to_string(),
                    artifact_id,
                })
                .padding([theme::SPACING_XS, theme::SPACING_SM])
                .style(theme::list_item)
                .width(Length::Fill);
                list = list.push(btn);
            }
            body = body.push(list);
        }
    }

    body = body
        .push(container(Space::new().height(1.0).width(Length::Fill)).style(column_divider))
        .push(
            text(exploration_line)
                .size(theme::font_sm())
                .color(theme::text_secondary()),
        )
        .push(
            text(change_line)
                .size(theme::font_sm())
                .color(theme::text_secondary()),
        )
        .push(Space::new().height(theme::SPACING_SM))
        .push(view_action_row(card, col));

    container(body)
        .padding(theme::SPACING_LG)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn view_action_row<'a>(card: &'a Card, col: Column) -> Element<'a, Message> {
    let lifecycle: Element<'a, Message> = if col.is_archived() {
        button(text("Unarchive").size(theme::font_md()))
            .on_press(Message::UnarchiveCard(card.id.clone()))
            .padding([theme::SPACING_XS, theme::SPACING_MD])
            .style(theme::list_item)
            .into()
    } else {
        button(text("Discard").size(theme::font_md()))
            .on_press(Message::DiscardCard(card.id.clone()))
            .padding([theme::SPACING_XS, theme::SPACING_MD])
            .style(theme::list_item)
            .into()
    };

    row![lifecycle, Space::new().width(Length::Fill)]
        .spacing(theme::SPACING_SM)
        .align_y(Center)
        .into()
}

fn column_label(col: Column) -> &'static str {
    match col {
        Column::Inbox => "Inbox",
        Column::Exploring => "Exploring",
        Column::Ready => "Ready",
        Column::InProgress => "In Progress",
        Column::Completed => "Completed",
        Column::ArchivedDone => "Archived (done)",
        Column::ArchivedAbandoned => "Archived (abandoned)",
        Column::ArchivedDiscarded => "Archived (discarded)",
    }
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
                a: 0.5,
                ..theme::bg_base()
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
            radius: 8.0.into(),
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
            radius: 4.0.into(),
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
    fn archived_done_when_change_archived() {
        let mut c = card();
        c.change_name = Some("ch".into());
        let mut archived = change("2026-04-24-01-ch");
        archived.prefix = "2026-04-24-01-".into();
        let p = project(vec![], vec![archived]);
        assert_eq!(classify(&c, &p, &[]), Column::ArchivedDone);
    }

    #[test]
    fn archived_abandoned_when_change_missing() {
        let mut c = card();
        c.change_name = Some("gone".into());
        let p = project(vec![], vec![]);
        assert_eq!(classify(&c, &p, &[]), Column::ArchivedAbandoned);
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
    fn archived_discarded_takes_priority() {
        let mut c = card();
        c.archived_at_nanos = Some(1);
        c.exploration_id = Some("e1".into());
        c.change_name = Some("ch".into());
        let p = project(vec![change("ch")], vec![]);
        assert_eq!(classify(&c, &p, &[exp("e1")]), Column::ArchivedDiscarded);
    }
}
