//! Focus-aware keybinding resolvers — one place to look for "what does this
//! action do given the current focus and area state?"
//!
//! Each `keybind_*` function is named for the *action* it represents, not the
//! key that triggers it today. Adding a new shortcut whose behavior depends
//! on focus belongs here; the dispatcher in `main::update` should stay a thin
//! `if mods.command() && key == … { keybind_thing(state) }` cascade so the
//! resolution rules don't drift back into the keypress arm.

use crate::State;
use crate::area::interaction::ActiveTab;
use crate::area::{self, Area};
use crate::widget::find::FindTarget;
use crate::widget::tab_bar::{self, ActiveTab as TabActive};

/// Which column the user last interacted with — drives focus-conditional
/// shortcut resolution (cmd+n, cmd+f, …). Updated lazily by
/// `main::update_focused_column` in response to chat/editor messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedColumn {
    Content,
    Chat,
}

// ── Action enums ────────────────────────────────────────────────────────────

/// What "new" should do given current focus + area.
#[derive(Debug, Clone)]
pub enum NewAction {
    /// Open the new-file modal. Seeded by the dispatcher with the focused
    /// editor tab's directory (or empty for project root).
    OpenNewFile,
    /// Add a new idea in the Ideas area.
    AddIdea,
    /// Spawn a new chat session in the active Change. Payload is the
    /// routing key the dispatcher already resolved.
    NewChatSession(String),
    /// Add a new exploration in the Change area.
    AddExploration,
}

/// What "save" should do. Currently only one focus-conditional flavor: the
/// Ideas pinned tab routes through the frontmatter-aware writer instead of
/// the generic file-save path.
#[derive(Debug, Clone)]
pub enum SaveAction {
    SaveIdeaBody,
}

// ── Resolvers ───────────────────────────────────────────────────────────────

/// `cmd+n` today. Content focus opens the new-file modal in every area;
/// otherwise area-scoped behavior takes over.
pub fn keybind_new(state: &State) -> Option<NewAction> {
    if state.project.project_root.is_none() {
        return None;
    }
    if state.focused_column == Some(FocusedColumn::Content) {
        return Some(NewAction::OpenNewFile);
    }
    match state.active_area {
        Area::Ideas => Some(NewAction::AddIdea),
        Area::Change => {
            let routing_key = state.active_interaction_key();
            let real_change_selected =
                routing_key.is_some() && !state.change.is_exploration_selected();
            if real_change_selected {
                routing_key.map(NewAction::NewChatSession)
            } else {
                Some(NewAction::AddExploration)
            }
        }
        _ => None,
    }
}

/// `cmd+s` today. The generic in-editor save flows through the editor's own
/// `SaveRequested` action; this resolver only flags the Ideas-pinned-tab
/// case which needs frontmatter-aware handling.
pub fn keybind_save(state: &State) -> Option<SaveAction> {
    if state.active_area == Area::Ideas
        && matches!(state.tabs.active, TabActive::Preview)
        && state
            .tabs
            .active_tab()
            .is_some_and(|t| t.id.starts_with(area::ideas::PINNED_TAB_PREFIX))
    {
        return Some(SaveAction::SaveIdeaBody);
    }
    None
}

/// `cmd+w` today. Returns the logical tab index to close (accounting for the
/// preview slot), or `None` when focus belongs to the chat input / terminal,
/// or when no closable tab is active.
pub fn keybind_close(state: &State) -> Option<usize> {
    if !matches!(
        state.active_area,
        Area::Change | Area::Caps | Area::Codex | Area::Ideas
    ) {
        return None;
    }
    let TabActive::File(fi) = state.tabs.active else {
        return None;
    };
    let chat_focused = state
        .active_scope()
        .and_then(|scope| state.interactions.get(&scope))
        .and_then(|ix| ix.active())
        .is_some_and(|ax| ax.chat_input_focused);
    let terminal_focused = state
        .active_scope()
        .and_then(|scope| state.interactions.get(&scope))
        .is_some_and(|ix| ix.terminal_focused);
    if chat_focused || terminal_focused {
        return None;
    }
    Some(if state.tabs.preview.is_some() {
        fi + 1
    } else {
        fi
    })
}

/// `cmd+k` today. True when the chat tab is the visible, active interaction
/// tab — the runtime check for "is there actually a tentative to pin" stays
/// at the call site since it needs `&mut`.
pub fn keybind_pin_selection(state: &State) -> bool {
    let Some(scope) = state.active_scope() else {
        return false;
    };
    let Some(ix) = state.interactions.get(&scope) else {
        return false;
    };
    ix.visible && ix.active_tab == ActiveTab::Chat && ix.active().is_some()
}

/// `cmd+r` today. True when chat input is focused and there's at least one
/// attachment (pinned or tentative) to clear.
pub fn keybind_clear_attachments(state: &State) -> bool {
    let Some(scope) = state.active_scope() else {
        return false;
    };
    let Some(ix) = state.interactions.get(&scope) else {
        return false;
    };
    if !ix.visible || ix.active_tab != ActiveTab::Chat {
        return false;
    }
    let Some(ax) = ix.active() else {
        return false;
    };
    ax.chat_input_focused && (!ax.selection_pinned.is_empty() || ax.selection_tentative.is_some())
}

/// `cmd+f` today. The local-find target for the focused column, or `None`
/// when neither column is in a state to host find (terminal focused, no
/// editor tab open, no chat session, search-stack active).
pub fn keybind_find(state: &State) -> Option<FindTarget> {
    match state.focused_column? {
        FocusedColumn::Content => {
            let tab = state.tabs.active_tab()?;
            match &tab.view {
                tab_bar::TabView::Editor { .. } | tab_bar::TabView::Diff { .. } => {
                    Some(FindTarget::editor(tab.id.clone()))
                }
                tab_bar::TabView::SearchStack { .. } => None,
            }
        }
        FocusedColumn::Chat => {
            let scope = state.active_scope()?;
            let ix = state.interactions.get(&scope)?;
            let ax = ix.active()?;
            Some(FindTarget::chat(ix.instance_id, ax.session.id.clone()))
        }
    }
}
