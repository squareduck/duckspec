//! Scope metadata used to build the per-session context hook.
//!
//! Every chat session belongs to a "scope" — a change, an exploration, the
//! capability tree, or the codex. Knowing which kind of scope is active lets
//! us prepend a short orientation line to the first turn so the agent doesn't
//! need to ask.
//!
//! Kept deliberately small: the hook only emits a few sentences. Rich context
//! (step files, diffs) belongs in separate hooks added on demand.
//!
//! Not persisted — inferred from the panel that owns the session at construction
//! time (caps / codex panels know their kind; change panels decide between
//! `Change` and `Exploration` based on the explorations list).

use duckchat::{ContextHook, HookOutput};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeKind {
    Change,
    Exploration,
    Caps,
    Codex,
}

/// Identity of an interaction column scope. Acts as the key for the global
/// `state.interactions` map and is computed from the active area + that
/// area's selection. The string variants carry the same value used as the
/// on-disk scope key (`ChatSession.scope`), so chat_store calls remain
/// straightforward.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Scope {
    Caps,
    Codex,
    Change(String),
    Exploration(String),
}

impl Scope {
    /// String key used by chat_store and on-disk paths. Matches the value
    /// stored in `ChatSession.scope`.
    pub fn key(&self) -> &str {
        match self {
            Scope::Caps => "caps",
            Scope::Codex => "codex",
            Scope::Change(name) => name.as_str(),
            Scope::Exploration(id) => id.as_str(),
        }
    }

    pub fn kind(&self) -> ScopeKind {
        match self {
            Scope::Caps => ScopeKind::Caps,
            Scope::Codex => ScopeKind::Codex,
            Scope::Change(_) => ScopeKind::Change,
            Scope::Exploration(_) => ScopeKind::Exploration,
        }
    }
}

/// Input the `CurrentScopeHook` reads. Built by `send_prompt_text` right
/// before dispatching the first turn of a session.
pub struct SessionScope {
    pub kind: ScopeKind,
    /// Scope key — the on-disk directory name, also stored in
    /// `ChatSession.scope`. For changes this equals the change name; for
    /// explorations it's the stable `exploration-{nanos}` id (once chunk 2
    /// lands).
    pub scope_key: String,
}

/// Prepends a short "this is what we're working on" blurb to the first turn
/// of each session. Subsequent turns ride the resumed Claude session, which
/// already has the blurb in its history.
pub struct CurrentScopeHook;

impl ContextHook<SessionScope> for CurrentScopeHook {
    fn name(&self) -> &str {
        "current-scope"
    }

    fn compute(&self, scope: &SessionScope) -> Option<HookOutput> {
        let text = match scope.kind {
            ScopeKind::Change => format!(
                "Current duckspec scope: change `{}`. Change artifacts live under `changes/{0}/`.",
                scope.scope_key
            ),
            ScopeKind::Exploration => {
                "Current duckspec scope: exploration — an informal brainstorming chat with no \
formal artifacts yet. Treat the conversation as early-stage scoping; don't expect \
a change directory to exist."
                    .to_string()
            }
            ScopeKind::Caps => {
                "Current duckspec scope: the project's capability tree (caps). See `caps.md` \
and `project.md` in the project root."
                    .to_string()
            }
            ScopeKind::Codex => {
                "Current duckspec scope: the project's codex. See `codex.md` in the project root."
                    .to_string()
            }
        };
        Some(HookOutput { text })
    }
}
