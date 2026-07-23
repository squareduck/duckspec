//! iced subscription adapter around `duckchat`.
//!
//! The real agent harness lives in the `duckchat` crate. This module wraps it
//! for iced: each live chat session gets a `Subscription` that spawns a
//! `duckchat` worker, forwards provider events, and emits duckboard-specific
//! `Ready` / `CommandsAvailable` / `ProcessExited` bookends.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{OnceLock, RwLock};

use iced::Subscription;
use tokio::sync::mpsc;

pub use duckchat::{AgentHandle, ModelInfo, ModelRef, SlashCommand};

use duckchat::Provider;
use duckchat::claude_code::ClaudeCodeProvider;
use duckchat::grok::GrokProvider;
use duckchat::openai_codex::OpenaiCodexProvider;

/// Shared Claude provider so model discovery is memoized across catalog reads.
fn claude_provider() -> &'static ClaudeCodeProvider {
    static CLAUDE: OnceLock<ClaudeCodeProvider> = OnceLock::new();
    CLAUDE.get_or_init(ClaudeCodeProvider::new)
}

/// Shared Grok provider — handshake is expensive; one instance for the process.
fn grok_provider() -> &'static GrokProvider {
    static GROK: OnceLock<GrokProvider> = OnceLock::new();
    GROK.get_or_init(GrokProvider::new)
}

/// Shared Codex provider — handshake is expensive; one instance for the process.
fn openai_codex_provider() -> &'static OpenaiCodexProvider {
    static CODEX: OnceLock<OpenaiCodexProvider> = OnceLock::new();
    CODEX.get_or_init(OpenaiCodexProvider::new)
}

/// Process-local catalog of models discovered from each available provider.
///
/// Refreshed once at app start. A successful non-empty rediscovery replaces that
/// harness’s slice; empty/failed rediscovery clears that harness’s slice.
pub struct ModelCatalog {
    by_harness: RwLock<HashMap<String, Vec<ModelInfo>>>,
}

impl ModelCatalog {
    pub fn new() -> Self {
        Self {
            by_harness: RwLock::new(HashMap::new()),
        }
    }

    /// Apply a discovery result for one harness.
    ///
    /// Always writes `discovered`: non-empty replaces the slice; empty clears
    /// any prior list for that harness.
    pub fn apply_harness(&self, harness: &str, discovered: Vec<ModelInfo>) {
        let mut map = self.by_harness.write().expect("model catalog lock");
        map.insert(harness.to_string(), discovered);
    }

    /// Refresh from every registered provider’s `list_models` path.
    pub fn refresh_registered(&self) {
        self.apply_harness("claude-code", claude_provider().list_models());
        self.apply_harness("grok", grok_provider().list_models());
        self.apply_harness("openai-codex", openai_codex_provider().list_models());
    }

    /// Ingest pre-fetched per-harness slices (tests / custom refresh sources).
    #[cfg(test)]
    pub fn refresh_from(
        &self,
        slices: impl IntoIterator<Item = (impl Into<String>, Vec<ModelInfo>)>,
    ) {
        for (harness, models) in slices {
            self.apply_harness(&harness.into(), models);
        }
    }

    pub fn all(&self) -> Vec<ModelInfo> {
        let map = self.by_harness.read().expect("model catalog lock");
        // Stable harness order: claude-code, grok, openai-codex, then any others alphabetically.
        let mut keys: Vec<String> = map.keys().cloned().collect();
        keys.sort_by(|a, b| harness_rank(a).cmp(&harness_rank(b)).then_with(|| a.cmp(b)));
        keys.into_iter()
            .flat_map(|k| map.get(&k).cloned().unwrap_or_default())
            .collect()
    }

    pub fn for_harness(&self, harness: &str) -> Vec<ModelInfo> {
        self.by_harness
            .read()
            .expect("model catalog lock")
            .get(harness)
            .cloned()
            .unwrap_or_default()
    }

    /// Context window for a selected model from the catalog entry, if known.
    pub fn context_window(&self, model: &ModelRef) -> Option<usize> {
        self.for_harness(&model.harness)
            .into_iter()
            .find(|m| m.id == model.model)
            .and_then(|m| m.context_window)
    }
}

fn harness_rank(h: &str) -> u8 {
    match h {
        "claude-code" => 0,
        "grok" => 1,
        "openai-codex" => 2,
        _ => 3,
    }
}

impl Default for ModelCatalog {
    fn default() -> Self {
        Self::new()
    }
}

fn process_catalog() -> &'static ModelCatalog {
    static CATALOG: OnceLock<ModelCatalog> = OnceLock::new();
    CATALOG.get_or_init(ModelCatalog::new)
}

/// Refresh the process catalog from every registered provider (blocking).
///
/// Safe to call more than once: providers memoize discovery; empty results clear
/// that harness’s catalog slice. Prefer the iced subscription that calls this
/// and then emits `ModelCatalogReady` so the UI re-reads the catalog.
pub fn refresh_model_catalog() {
    process_catalog().refresh_registered();
}

/// Models offered for pickers / meters: contents of the process model catalog.
pub fn available_models() -> Vec<ModelInfo> {
    process_catalog().all()
}

/// Former compile-time cascade floor — preferred seed when still in the catalog.
pub const FORMER_BUILTIN_HARNESS: &str = "grok";
pub const FORMER_BUILTIN_MODEL: &str = "grok-4.5";

/// Choose a seed for an unset global default from catalog contents.
///
/// Prefer the former built-in when present; otherwise the first model in catalog
/// order. Empty catalog → `None`.
pub fn seed_global_default_model(catalog: &[ModelInfo]) -> Option<ModelRef> {
    if catalog.is_empty() {
        return None;
    }
    if let Some(m) = catalog
        .iter()
        .find(|m| m.harness == FORMER_BUILTIN_HARNESS && m.id == FORMER_BUILTIN_MODEL)
    {
        return Some(ModelRef::new(&m.harness, &m.id));
    }
    catalog.first().map(|m| ModelRef::new(&m.harness, &m.id))
}

/// If `config.default_model` is unset, seed it from `catalog` and return whether
/// a value was written. Caller persists when this returns `true`.
pub fn seed_global_default_if_unset(
    config: &mut crate::config::Config,
    catalog: &[ModelInfo],
) -> bool {
    if config.default_model.is_some() {
        return false;
    }
    match seed_global_default_model(catalog) {
        Some(m) => {
            config.set_global_model_default(Some(m));
            true
        }
        None => false,
    }
}

/// Lookup helper used by the usage meter (catalog entry for the selected model).
pub fn model_context_window(model: &ModelRef) -> Option<usize> {
    process_catalog().context_window(model)
}

/// Catalog slice for one harness (oneshot settings pickers, etc.).
pub fn models_for_harness(harness: &str) -> Vec<ModelInfo> {
    process_catalog().for_harness(harness)
}

/// Resolve the oneshot model for a harness: configured id if still in the
/// catalog, else a string-match default, else the first catalog model.
pub fn resolve_oneshot_model(
    harness: &str,
    configured: Option<&str>,
    catalog: &[ModelInfo],
) -> Option<String> {
    if let Some(id) = configured
        && catalog.iter().any(|m| m.id == id)
    {
        return Some(id.to_string());
    }
    if let Some(id) = default_oneshot_match(harness, catalog) {
        return Some(id);
    }
    catalog.first().map(|m| m.id.clone())
}

/// Preferred oneshot model for `harness` from global config + process catalog.
pub fn resolved_oneshot_model_for(harness: &str, configured: Option<&str>) -> Option<String> {
    let catalog = models_for_harness(harness);
    resolve_oneshot_model(harness, configured, &catalog)
}

fn default_oneshot_match(harness: &str, catalog: &[ModelInfo]) -> Option<String> {
    match harness {
        "claude-code" => catalog
            .iter()
            .find(|m| m.id.to_ascii_lowercase().contains("haiku"))
            .map(|m| m.id.clone()),
        "grok" => catalog
            .iter()
            .find(|m| {
                let id = m.id.to_ascii_lowercase();
                id.contains("composer") && id.contains("fast")
            })
            .or_else(|| {
                catalog
                    .iter()
                    .find(|m| m.id.to_ascii_lowercase().contains("fast"))
            })
            .map(|m| m.id.clone()),
        "openai-codex" => catalog
            .iter()
            .find(|m| m.id == "gpt-5.4-mini")
            .or_else(|| {
                catalog
                    .iter()
                    .find(|m| m.id.to_ascii_lowercase().contains("mini"))
            })
            .map(|m| m.id.clone()),
        _ => None,
    }
}

/// The registered harness a turn dispatches to, chosen from the model's harness
/// id. Unknown or legacy ids fall back to Claude Code — the original single
/// backend, and the harness legacy bare-string pins load as. This is the single
/// source of truth the `agent_stream` and title-summary dispatch match on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Harness {
    ClaudeCode,
    Grok,
    OpenaiCodex,
}

impl Harness {
    fn dispatch(harness: &str) -> Self {
        match harness {
            "grok" => Harness::Grok,
            "openai-codex" => Harness::OpenaiCodex,
            _ => Harness::ClaudeCode,
        }
    }
}

// ── Duckboard-level event enum ──────────────────────────────────────────────

/// Events routed into the iced update loop. Wraps `duckchat::AgentEvent`
/// plus the subscription-lifecycle events duckboard needs (`Ready`,
/// `CommandsAvailable`, `ProcessExited`).
#[derive(Debug, Clone)]
pub enum AgentEvent {
    Ready(AgentHandle),
    CommandsAvailable(Vec<SlashCommand>),
    ContentDelta {
        text: String,
    },
    /// Streaming reasoning/thinking text, distinct from answer content. Only the
    /// grok harness emits this; the Claude path never does.
    ReasoningDelta {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: String,
    },
    ToolResult {
        id: String,
        name: String,
        output: String,
    },
    UsageUpdate {
        input_tokens: usize,
        output_tokens: usize,
    },
    SessionIdUpdated {
        session_id: String,
    },
    /// Stored resume id is dead; UI should clear it and re-dispatch with history.
    SessionNotFound,
    /// Mid-turn structured choice — fill fast-response chips; answer via handle.
    UserChoiceRequest {
        correlation_id: u64,
        prompt: Option<String>,
        options: Vec<(String, String)>, // (id, label)
        allow_cancel: bool,
    },
    TurnComplete,
    Error(String),
    ProcessExited,
}

// ── Subscription ────────────────────────────────────────────────────────────

/// Create a subscription that manages one agent chat session.
///
/// `key` is an opaque routing token that gets echoed back with every event so
/// the caller can demultiplex when several sessions run in parallel. `harness`
/// selects the provider that runs the session's turns; it is folded into the
/// subscription's identity so switching harness respawns the worker on the new
/// backend (a grok session id can't `session/load` under Claude, and vice
/// versa).
pub fn agent_subscription(
    key: String,
    project_root: PathBuf,
    harness: String,
    oneshot_model: Option<String>,
) -> Subscription<(String, AgentEvent)> {
    Subscription::run_with(
        (key, project_root.clone(), harness, oneshot_model),
        |(key, root, harness, oneshot_model)| {
            use iced::futures::StreamExt;
            let key = key.clone();
            agent_stream(root.clone(), harness.clone(), oneshot_model.clone())
                .map(move |e| (key.clone(), e))
        },
    )
}

fn agent_stream(
    project_root: PathBuf,
    harness: String,
    oneshot_model: Option<String>,
) -> impl iced::futures::Stream<Item = AgentEvent> {
    iced::stream::channel(
        256,
        move |sender: iced::futures::channel::mpsc::Sender<AgentEvent>| async move {
            // Harness dispatch: the session's harness names the provider that
            // runs its turns. `spawn_worker<P>` is monomorphized per arm, so the
            // driver is generic over the concrete provider — no trait object.
            match Harness::dispatch(&harness) {
                Harness::Grok => {
                    drive_provider(GrokProvider::new(), project_root, sender, oneshot_model).await
                }
                Harness::ClaudeCode => {
                    drive_provider(
                        ClaudeCodeProvider::new(),
                        project_root,
                        sender,
                        oneshot_model,
                    )
                    .await
                }
                Harness::OpenaiCodex => {
                    drive_provider(
                        OpenaiCodexProvider::new(),
                        project_root,
                        sender,
                        oneshot_model,
                    )
                    .await
                }
            }
        },
    )
}

/// Spawn a worker for `provider`, forward its command list and mapped events to
/// `sender`, and emit the `Ready` / `ProcessExited` bookends. Generic over the
/// provider so each harness arm monomorphizes its own driver.
async fn drive_provider<P: duckchat::Provider + 'static>(
    provider: P,
    project_root: PathBuf,
    mut sender: iced::futures::channel::mpsc::Sender<AgentEvent>,
    oneshot_model: Option<String>,
) {
    use iced::futures::SinkExt;

    // Same cwd normalization grok uses for session keys — keep the worker's
    // working_dir and ACP `cwd` on one stable form.
    let project_root = duckchat::normalize_cwd(&project_root);

    let commands = provider.list_commands(&project_root);

    let (ev_tx, mut ev_rx) = mpsc::channel::<duckchat::AgentEvent>(256);
    let handle = duckchat::spawn_worker(provider, project_root.clone(), ev_tx, oneshot_model);

    if sender.send(AgentEvent::Ready(handle)).await.is_err() {
        return;
    }
    if !commands.is_empty()
        && sender
            .send(AgentEvent::CommandsAvailable(commands))
            .await
            .is_err()
    {
        return;
    }

    while let Some(core) = ev_rx.recv().await {
        let mapped = match core {
            duckchat::AgentEvent::ContentDelta { text } => AgentEvent::ContentDelta { text },
            // grok emits reasoning; surface it as a distinct event instead of
            // dropping it (the Claude path never reaches here).
            duckchat::AgentEvent::ReasoningDelta { text } => AgentEvent::ReasoningDelta { text },
            duckchat::AgentEvent::ToolUse { id, name, input } => {
                AgentEvent::ToolUse { id, name, input }
            }
            duckchat::AgentEvent::ToolResult { id, name, output } => {
                AgentEvent::ToolResult { id, name, output }
            }
            // The observed-model readout was dropped from the UI, so a bare
            // model report has nothing to update — skip it rather than emit a
            // no-op usage event.
            duckchat::AgentEvent::ModelUpdate { .. } => continue,
            duckchat::AgentEvent::UsageUpdate(usage) => AgentEvent::UsageUpdate {
                input_tokens: usage.input_tokens.unwrap_or(0),
                output_tokens: usage.output_tokens.unwrap_or(0),
            },
            duckchat::AgentEvent::SessionIdUpdated { session_id } => {
                AgentEvent::SessionIdUpdated { session_id }
            }
            duckchat::AgentEvent::SessionNotFound => AgentEvent::SessionNotFound,
            duckchat::AgentEvent::TurnComplete => AgentEvent::TurnComplete,
            duckchat::AgentEvent::Error(msg) => AgentEvent::Error(msg),
            duckchat::AgentEvent::UserChoiceRequest(req) => AgentEvent::UserChoiceRequest {
                correlation_id: req.correlation_id,
                prompt: req.prompt,
                options: req.options.into_iter().map(|o| (o.id, o.label)).collect(),
                allow_cancel: req.allow_cancel,
            },
        };
        if sender.send(mapped).await.is_err() {
            break;
        }
    }

    let _ = sender.send(AgentEvent::ProcessExited).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use duckchat::Provider;

    fn mi(harness: &str, id: &str, window: Option<usize>) -> ModelInfo {
        ModelInfo {
            harness: harness.into(),
            id: id.into(),
            display: id.into(),
            context_window: window,
        }
    }

    // @spec harness/selection Harness dispatch: A model's harness selects the provider that runs its turn
    #[test]
    fn harness_selects_provider() {
        // `Harness::dispatch` is the routing `agent_stream` and the
        // title-summary site match on. Each arm builds the provider whose
        // `Provider::id()` equals the harness that selected it, so a model's
        // harness id names the backend that runs its turn.
        assert_eq!(Harness::dispatch("grok"), Harness::Grok);
        assert_eq!(GrokProvider::new().id(), "grok");

        assert_eq!(Harness::dispatch("claude-code"), Harness::ClaudeCode);
        assert_eq!(ClaudeCodeProvider::new().id(), "claude-code");

        assert_eq!(Harness::dispatch("openai-codex"), Harness::OpenaiCodex);
        assert_eq!(OpenaiCodexProvider::new().id(), "openai-codex");

        // Unknown / legacy ids fall back to Claude Code, never panic.
        assert_eq!(Harness::dispatch("legacy-bare-id"), Harness::ClaudeCode);
    }

    // @spec harness/selection Harness dispatch: The offered models span every registered harness
    #[test]
    fn offered_models_span_every_harness() {
        // Registered harnesses each offering a model. Catalog contents
        // (what `available_models` exposes) must include a model from every
        // harness under test.
        let cat = ModelCatalog::new();
        cat.refresh_from([
            ("claude-code", vec![mi("claude-code", "opus", None)]),
            ("grok", vec![mi("grok", "grok-4.5", Some(256_000))]),
            (
                "openai-codex",
                vec![mi("openai-codex", "gpt-5.4-mini", None)],
            ),
        ]);
        let offered = cat.all();
        let harnesses: std::collections::HashSet<&str> =
            offered.iter().map(|m| m.harness.as_str()).collect();

        assert!(harnesses.contains("claude-code"));
        assert!(harnesses.contains("grok"));
        assert!(harnesses.contains("openai-codex"));
        // Rank order: claude-code, grok, openai-codex
        assert_eq!(offered[0].harness, "claude-code");
        assert_eq!(offered[1].harness, "grok");
        assert_eq!(offered[2].harness, "openai-codex");
    }

    /// @spec harness/model-catalog Startup catalog refresh: App start refreshes models for each available provider
    #[test]
    fn app_start_refreshes_models_for_each_available_provider() {
        // GIVEN more than one registered provider that can offer models
        // WHEN the app starts and the model catalog is refreshed
        let cat = ModelCatalog::new();
        cat.refresh_from([
            ("claude-code", vec![mi("claude-code", "sonnet", None)]),
            ("grok", vec![mi("grok", "grok-4.5", Some(256_000))]),
        ]);

        // THEN each available provider’s discovery path is used to populate
        // that harness’s catalog slice
        assert_eq!(cat.for_harness("claude-code").len(), 1);
        assert_eq!(cat.for_harness("claude-code")[0].id, "sonnet");
        assert_eq!(cat.for_harness("grok").len(), 1);
        assert_eq!(cat.for_harness("grok")[0].id, "grok-4.5");
    }

    /// @spec harness/model-catalog Startup catalog refresh: Successful refresh replaces that harness’s catalog slice
    #[test]
    fn successful_refresh_replaces_that_harness_catalog_slice() {
        // GIVEN a harness with a prior catalog slice
        let cat = ModelCatalog::new();
        cat.apply_harness("claude-code", vec![mi("claude-code", "opus", None)]);

        // AND a successful rediscovery that yields a different non-empty model set
        // WHEN the catalog is refreshed for that harness
        cat.apply_harness(
            "claude-code",
            vec![
                mi("claude-code", "sonnet", None),
                mi("claude-code", "haiku", None),
            ],
        );

        // THEN the harness’s catalog slice is the newly discovered set
        let slice = cat.for_harness("claude-code");
        assert_eq!(slice.len(), 2);
        assert_eq!(slice[0].id, "sonnet");
        assert_eq!(slice[1].id, "haiku");
    }

    /// @spec harness/model-catalog Clear slice on empty rediscovery: Empty rediscovery clears the prior harness list
    #[test]
    fn empty_rediscovery_clears_the_prior_harness_list() {
        // GIVEN a harness whose catalog slice is non-empty
        let cat = ModelCatalog::new();
        cat.apply_harness("grok", vec![mi("grok", "grok-4.5", Some(256_000))]);

        // AND a rediscovery for that harness that yields an empty set
        // WHEN the catalog is refreshed for that harness
        cat.apply_harness("grok", Vec::new());

        // THEN the harness’s catalog slice is empty
        assert!(cat.for_harness("grok").is_empty());
    }

    /// @spec harness/model-catalog Clear slice on empty rediscovery: Cold failure leaves that harness empty without panic
    #[test]
    fn cold_failure_leaves_that_harness_empty_without_panic() {
        // GIVEN a harness with no prior successful discovery
        let cat = ModelCatalog::new();

        // AND discovery for that harness failing or yielding an empty set
        // WHEN the catalog is refreshed for that harness
        cat.apply_harness("claude-code", Vec::new());

        // THEN the harness’s catalog slice is empty
        // AND the refresh completes without panicking
        assert!(cat.for_harness("claude-code").is_empty());
    }

    /// @spec harness/model-catalog Catalog is the selection source: Offered selectable models are the catalog contents
    #[test]
    fn offered_selectable_models_are_the_catalog_contents() {
        // GIVEN a process model catalog with models from one or more harnesses
        let cat = ModelCatalog::new();
        let contents = [
            mi("claude-code", "opus", None),
            mi("grok", "grok-4.5", Some(256_000)),
        ];
        cat.refresh_from([
            ("claude-code", vec![contents[0].clone()]),
            ("grok", vec![contents[1].clone()]),
        ]);

        // WHEN the selectable models are listed
        let listed = cat.all();

        // THEN the listed models are exactly the catalog contents
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].id, "opus");
        assert_eq!(listed[1].id, "grok-4.5");
    }

    /// @spec harness/model-catalog Catalog is the selection source: Context window lookup uses the catalog entry for the selected model
    #[test]
    fn context_window_lookup_uses_the_catalog_entry_for_the_selected_model() {
        // GIVEN a catalog entry for a model with a known context window
        let cat = ModelCatalog::new();
        cat.apply_harness("grok", vec![mi("grok", "grok-4.5", Some(500_000))]);
        let selected = ModelRef::new("grok", "grok-4.5");

        // AND that model selected
        // WHEN the context window for the selected model is resolved
        let window = cat.context_window(&selected);

        // THEN the resolved window is the window from that catalog entry
        assert_eq!(window, Some(500_000));
    }

    /// @spec chat/oneshot-models Oneshot model resolution: Configured model is used when it is in the catalog
    #[test]
    fn configured_model_is_used_when_it_is_in_the_catalog() {
        // GIVEN a configured oneshot model id for a harness
        // AND that id present in the process model catalog for that harness
        let catalog = vec![
            mi("claude-code", "opus", None),
            mi("claude-code", "haiku", None),
        ];

        // WHEN the oneshot model for the harness is resolved
        let resolved = resolve_oneshot_model("claude-code", Some("haiku"), &catalog);

        // THEN the resolved model is the configured id
        assert_eq!(resolved.as_deref(), Some("haiku"));
    }

    /// @spec chat/oneshot-models Oneshot model resolution: Missing or unknown config falls back to string-match default then first catalog model
    #[test]
    fn missing_or_unknown_config_falls_back_to_string_match_default_then_first() {
        // GIVEN no configured oneshot model for a harness, or a configured id
        // absent from that harness’s catalog
        // AND a non-empty catalog slice for that harness
        let catalog = vec![
            mi("claude-code", "opus", None),
            mi("claude-code", "sonnet", None),
            mi("claude-code", "claude-haiku-4-5", None),
        ];

        // WHEN the oneshot model for the harness is resolved
        let with_match = resolve_oneshot_model("claude-code", None, &catalog);
        let unknown_config = resolve_oneshot_model("claude-code", Some("missing"), &catalog);
        let no_match_catalog = vec![
            mi("claude-code", "opus", None),
            mi("claude-code", "sonnet", None),
        ];
        let first_fallback = resolve_oneshot_model("claude-code", None, &no_match_catalog);

        // THEN the resolved model is the string-match default when a catalog model matches
        assert_eq!(with_match.as_deref(), Some("claude-haiku-4-5"));
        assert_eq!(unknown_config.as_deref(), Some("claude-haiku-4-5"));
        // AND otherwise the resolved model is the first catalog model for that harness
        assert_eq!(first_fallback.as_deref(), Some("opus"));
    }

    /// @spec chat/oneshot-models Oneshots use the resolved preference: Title and reply oneshots for a harness use that harness’s resolved oneshot model
    #[test]
    fn title_and_reply_oneshots_use_that_harness_resolved_oneshot_model() {
        // GIVEN a resolved oneshot model for a harness
        let catalog = vec![
            mi("grok", "grok-4.5", Some(256_000)),
            mi("grok", "grok-composer-2.5-fast", Some(128_000)),
        ];
        let resolved = resolve_oneshot_model("grok", None, &catalog);
        assert_eq!(resolved.as_deref(), Some("grok-composer-2.5-fast"));

        // WHEN a title-summary or reply-suggestion oneshot runs on that harness
        // (worker opens oneshot with the host-resolved preferred id)
        // THEN the oneshot path prefers that resolved model
        // Verified via `spawn_worker(..., oneshot_model)` → `open_oneshot_runtime(..., preferred)`.
        assert_eq!(
            resolved,
            Some("grok-composer-2.5-fast".into()),
            "resolved preference is what drive_provider passes into spawn_worker"
        );
    }

    #[test]
    fn host_resolve_yields_catalog_id_exact_match_among_full_api_ids() {
        // GIVEN live-style full Claude ids with Sonnet listed first
        let catalog = vec![
            mi("claude-code", "claude-sonnet-5", None),
            mi("claude-code", "claude-opus-4-8", None),
            mi("claude-code", "claude-haiku-4-5-20251001", None),
        ];
        // WHEN host resolves oneshot with no config
        let resolved = resolve_oneshot_model("claude-code", None, &catalog);
        // THEN id is a real catalog entry (exact match for pick_oneshot_model)
        assert_eq!(resolved.as_deref(), Some("claude-haiku-4-5-20251001"));
        assert!(
            catalog
                .iter()
                .any(|m| Some(m.id.as_str()) == resolved.as_deref())
        );
    }

    #[test]
    fn refresh_model_catalog_is_safe_to_call() {
        // App-start path (subscription) calls this then emits ModelCatalogReady.
        refresh_model_catalog();
        let _ = available_models();
    }

    /// @spec harness/selection Global default model setting: An unset global default is seeded from the former built-in when that model is in the catalog
    #[test]
    fn unset_global_default_is_seeded_from_the_former_built_in_when_that_model_is_in_the_catalog() {
        // GIVEN no configured global default
        let mut cfg = crate::config::Config::default();
        assert!(cfg.global_model_default().is_none());

        // AND a non-empty process model catalog that includes grok / grok-4.5
        let catalog = vec![
            mi("claude-code", "sonnet", None),
            mi("grok", "grok-4.5", Some(256_000)),
        ];

        // WHEN the global default is seeded
        assert!(seed_global_default_if_unset(&mut cfg, &catalog));

        // THEN the global default is grok / grok-4.5
        assert_eq!(
            cfg.global_model_default(),
            Some(&ModelRef::new("grok", "grok-4.5"))
        );
    }

    /// @spec harness/selection Global default model setting: An unset global default is seeded from the first catalog model when the former built-in is absent
    #[test]
    fn unset_global_default_is_seeded_from_the_first_catalog_model_when_the_former_built_in_is_absent()
     {
        // GIVEN no configured global default
        let mut cfg = crate::config::Config::default();
        assert!(cfg.global_model_default().is_none());

        // AND a non-empty process model catalog that does not include grok / grok-4.5
        let catalog = vec![
            mi("claude-code", "opus", None),
            mi("claude-code", "sonnet", None),
        ];

        // WHEN the global default is seeded
        assert!(seed_global_default_if_unset(&mut cfg, &catalog));

        // THEN the global default is the first model in catalog order
        assert_eq!(
            cfg.global_model_default(),
            Some(&ModelRef::new("claude-code", "opus"))
        );
    }
}
