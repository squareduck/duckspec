//! Context hooks contribute extra system-prompt fragments to a turn.
//!
//! A hook is a pure function of some caller-supplied scope (e.g. the current
//! duckspec change, the active step file, the current git diff). The caller
//! invokes hooks before `Provider::run_turn` and pushes their outputs into
//! `TurnRequest::system_additions`.
//!
//! The trait is generic over the scope type because different callers build
//! different scopes. duckchat itself doesn't run hooks — it only defines the
//! shape so multiple callers can share a vocabulary.

/// A single hook contribution. Currently just text; future variants could
/// carry attachments or tool-availability toggles.
#[derive(Debug, Clone)]
pub struct HookOutput {
    pub text: String,
}

pub trait ContextHook<S>: Send + Sync {
    fn name(&self) -> &str;
    fn compute(&self, scope: &S) -> Option<HookOutput>;
}
