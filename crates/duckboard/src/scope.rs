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

use std::path::PathBuf;

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
    /// Lifecycle facts for a change scope, carried from the session via
    /// `AgentSession.scope_facts`. `None` for non-change scopes, and also for a
    /// change with no recoverable facts (archived/unknown). The `Change` arm of
    /// the hook renders progress and the next stage from these.
    pub change_facts: Option<crate::area::change::ChangeScopeFacts>,
}

/// Render the authoritative orientation for a change scope: name the change,
/// report step progress (with the active step's task tally when present), name
/// the suggested next stage, and assert that change-acting commands default to
/// this change. Falls back to a name-only blurb when the change carries no
/// lifecycle facts (archived/unknown), but still asserts the default target.
fn render_change_orientation(scope: &SessionScope) -> String {
    let name = &scope.scope_key;
    let authority = "Change-acting commands (like /ds-apply and /ds-archive) act on THIS \
change by default — only disambiguate via `ds status` if the user names a different \
change.";

    let Some(facts) = &scope.change_facts else {
        return format!(
            "Current duckspec scope: change `{name}`. Change artifacts live under \
`changes/{name}/`. {authority}"
        );
    };

    // Step-level progress. `step_count == 0` means no steps yet — the phase
    // label already describes that state, so report no count. A change whose
    // steps are all done reads as complete; otherwise note the active step's
    // task tally when one is in flight.
    let progress = if facts.step_count == 0 {
        format!("Phase: {}.", facts.phase)
    } else if facts.steps_done == facts.step_count {
        format!(
            "Phase: {} (all {} steps complete).",
            facts.phase, facts.step_count
        )
    } else {
        let tally = match facts.active_step_tasks {
            Some((done, total)) => format!("; active step {done}/{total} tasks"),
            None => String::new(),
        };
        format!(
            "Phase: {} ({} of {} steps done{tally}).",
            facts.phase, facts.steps_done, facts.step_count
        )
    };

    let next = match &facts.next_command {
        Some(cmd) => format!(" Suggested next stage: /{cmd}."),
        None => String::new(),
    };

    format!(
        "Current duckspec scope: change `{name}`. Change artifacts live under \
`changes/{name}/`. {progress}{next} {authority}"
    )
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
            ScopeKind::Change => render_change_orientation(scope),
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

/// Inject the project's `AGENTS.md` (if present) into the first turn so the
/// agent picks up project conventions. Claude Code natively auto-discovers
/// `CLAUDE.md` but not `AGENTS.md`; this hook bridges that gap and works for
/// any agent backend we might add later.
pub struct AgentsMarkdownHook;

/// Cap on AGENTS.md content we'll inject. Keeps a runaway file from blowing
/// out the first-turn system prompt.
const AGENTS_MD_CHAR_CAP: usize = 16_000;

impl ContextHook<PathBuf> for AgentsMarkdownHook {
    fn name(&self) -> &str {
        "agents-md"
    }

    fn compute(&self, project_root: &PathBuf) -> Option<HookOutput> {
        let path = project_root.join("AGENTS.md");
        let raw = std::fs::read_to_string(&path).ok()?;
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return None;
        }
        let body = if trimmed.chars().count() > AGENTS_MD_CHAR_CAP {
            let end = trimmed
                .char_indices()
                .nth(AGENTS_MD_CHAR_CAP)
                .map(|(i, _)| i)
                .unwrap_or(trimmed.len());
            &trimmed[..end]
        } else {
            trimmed
        };
        Some(HookOutput {
            text: format!(
                "Project conventions from `AGENTS.md` (project root). Treat these as standing \
instructions for this repository:\n\n{body}"
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::area::change::ChangeScopeFacts;
    use duckchat::ContextHook;

    fn orientation(scope: &SessionScope) -> String {
        CurrentScopeHook
            .compute(scope)
            .expect("scope hook always produces orientation")
            .text
    }

    /// @spec session/scope Change identification and authority: Orientation names the scoped change as the default command target
    #[test]
    fn change_orientation_names_change_and_asserts_default_target() {
        let scope = SessionScope {
            kind: ScopeKind::Change,
            scope_key: "foo".into(),
            change_facts: Some(ChangeScopeFacts {
                phase: "implementing steps",
                steps_done: 1,
                step_count: 3,
                active_step_tasks: Some((2, 5)),
                next_command: Some("ds-apply".into()),
            }),
        };
        let text = orientation(&scope);
        assert!(
            text.contains("`foo`"),
            "orientation must name the scoped change: {text}"
        );
        assert!(
            text.contains("act on THIS change by default"),
            "orientation must establish the change as the default command target: {text}"
        );
        assert!(
            text.contains("names a different change"),
            "orientation must direct disambiguation to the different-change case: {text}"
        );
    }

    /// @spec session/scope Non-change scope orientation: An exploration scope signals early-stage work with no change artifacts
    #[test]
    fn exploration_orientation_is_early_stage_with_no_change_facts() {
        let scope = SessionScope {
            kind: ScopeKind::Exploration,
            scope_key: "exploration-123".into(),
            change_facts: None,
        };
        let text = orientation(&scope);
        assert!(
            text.contains("early-stage"),
            "exploration orientation should signal early-stage work: {text}"
        );
        assert!(
            !text.contains("Suggested next stage"),
            "exploration orientation must not report a change next-stage: {text}"
        );
        assert!(
            !text.contains("steps done"),
            "exploration orientation must not report change progress: {text}"
        );
    }

    /// @spec session/scope Non-change scope orientation: A capability-tree scope carries no change facts
    #[test]
    fn caps_orientation_describes_tree_with_no_change_facts() {
        let scope = SessionScope {
            kind: ScopeKind::Caps,
            scope_key: "caps".into(),
            change_facts: None,
        };
        let text = orientation(&scope);
        assert!(
            text.contains("capability tree"),
            "caps orientation should describe the capability tree: {text}"
        );
        assert!(
            !text.contains("Suggested next stage"),
            "caps orientation must not report a change next-stage: {text}"
        );
        assert!(
            !text.contains("steps done"),
            "caps orientation must not report change progress: {text}"
        );
    }
}
