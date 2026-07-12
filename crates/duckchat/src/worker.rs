//! Long-lived worker that owns a main runtime and a oneshot runtime per chat.
//!
//! Main turns are serialised on one loop; title/reply oneshots share a second
//! concurrent loop so they are not blocked by an in-flight turn. Cancel only
//! affects the main path; shutdown tears down both runtimes.

use std::path::PathBuf;
use std::time::Duration;

use tokio::sync::{mpsc, oneshot};
use tokio::time::timeout;

use crate::cancel::CancelToken;
use crate::error::Error;
use crate::event::{AgentEvent, PendingUserChoices, UserChoiceAnswer};
use crate::provider::Provider;
use crate::reply_suggest::{
    REPLY_SUGGEST_INSTRUCTION, build_reply_suggest_prompt, parse_replies, should_skip_model,
};
use crate::request::{ReplySuggestionRequest, TitleRequest, TurnRequest};
use crate::runtime::OneshotKind;
use crate::title::{build_title_prompt, clean_title};

/// Wall-clock oneshot call budget for one Work item (`ensure_hot` + `prompt`).
/// Matches the warm-runtime contract (thirty seconds of wall-clock time).
pub const ONESHOT_CALL_BUDGET: Duration = Duration::from_secs(30);

/// Commands the caller can queue on the main worker loop.
#[derive(Debug)]
pub enum AgentCommand {
    /// Run a prompt turn. Convenience helpers on [`AgentHandle`] construct
    /// this.
    RunTurn(TurnRequest),
    /// Answer a parked mid-turn user choice. Prefer
    /// [`AgentHandle::answer_user_choice`] (side channel) while a turn is
    /// in flight — the main command loop is blocked on `run_turn`.
    AnswerUserChoice {
        correlation_id: u64,
        answer: UserChoiceAnswer,
    },
    /// Seed the session id used by the next turn. Useful when resuming a
    /// previously-persisted conversation — the caller knows the id before the
    /// worker has seen a turn.
    SetSessionId(String),
    /// Forget any stored resume id (e.g. after [`AgentEvent::SessionNotFound`]).
    ClearSessionId,
    /// Stop processing further commands and return.
    Shutdown,
}

/// Internal oneshot-path command (not part of the public command API).
enum OneshotCommand {
    /// Best-effort `ensure_hot` (e.g. first main turn kicks oneshot warm-up).
    Warm,
    /// Run one isolated prompt and reply on the oneshot channel.
    Work(OneshotRequest),
    /// Shut down the oneshot runtime and exit the loop.
    Shutdown,
}

struct OneshotRequest {
    kind: OneshotKind,
    prompt: String,
    reply: oneshot::Sender<Result<String, Error>>,
}

/// Cloneable handle for driving a worker.
#[derive(Clone)]
pub struct AgentHandle {
    cancel: CancelToken,
    tx: mpsc::UnboundedSender<AgentCommand>,
    oneshot_tx: mpsc::UnboundedSender<OneshotCommand>,
    working_dir: PathBuf,
    /// Side channel for mid-turn choices (works while `run_turn` is blocked).
    pending_choices: std::sync::Arc<PendingUserChoices>,
}

impl AgentHandle {
    /// Queue a `TurnRequest` directly.
    pub fn send_turn(&self, req: TurnRequest) {
        let _ = self.tx.send(AgentCommand::RunTurn(req));
    }

    /// Convenience: build a minimal `TurnRequest` from `prompt` using the
    /// handle's working dir and queue it.
    pub fn send_prompt(&self, prompt: String) {
        self.send_turn(TurnRequest::new(prompt, self.working_dir.clone()));
    }

    pub fn set_session_id(&self, session_id: String) {
        let _ = self.tx.send(AgentCommand::SetSessionId(session_id));
    }

    /// Drop any worker-side resume id so the next turn opens a fresh session.
    pub fn clear_session_id(&self) {
        let _ = self.tx.send(AgentCommand::ClearSessionId);
    }

    /// Cancel the in-flight main turn. Does not tear down the oneshot path.
    /// Also completes any parked user choice as cancelled.
    pub fn cancel(&self) {
        self.pending_choices.cancel_all();
        self.cancel.cancel();
    }

    /// Answer a parked mid-turn [`AgentEvent::UserChoiceRequest`].
    /// Uses a side channel so it works while the main turn is in flight.
    pub fn answer_user_choice(&self, correlation_id: u64, answer: UserChoiceAnswer) {
        self.pending_choices.answer(correlation_id, answer);
    }

    pub fn shutdown(&self) {
        self.pending_choices.cancel_all();
        self.cancel.cancel();
        let _ = self.tx.send(AgentCommand::Shutdown);
    }

    pub fn working_dir(&self) -> &std::path::Path {
        &self.working_dir
    }

    /// Title summary via the shared oneshot runtime (serialized with reply suggestions).
    pub async fn title_summary(&self, req: TitleRequest) -> Result<String, Error> {
        let text = build_title_prompt(&req);
        let raw = self.oneshot_call(OneshotKind::Title, text).await?;
        Ok(clean_title(&raw))
    }

    /// Reply suggestions via the same oneshot runtime. Empty assistant short-circuits
    /// without a model call.
    pub async fn reply_suggestions(
        &self,
        req: ReplySuggestionRequest,
    ) -> Result<Vec<String>, Error> {
        if should_skip_model(&req) {
            return Ok(Vec::new());
        }
        let body = build_reply_suggest_prompt(&req);
        let text = format!("{REPLY_SUGGEST_INSTRUCTION}\n\n{body}");
        let raw = self.oneshot_call(OneshotKind::ReplySuggest, text).await?;
        Ok(parse_replies(&raw))
    }

    async fn oneshot_call(&self, kind: OneshotKind, prompt: String) -> Result<String, Error> {
        let (tx, rx) = oneshot::channel();
        self.oneshot_tx
            .send(OneshotCommand::Work(OneshotRequest {
                kind,
                prompt,
                reply: tx,
            }))
            .map_err(|_| Error::Other("agent worker gone".into()))?;
        rx.await
            .map_err(|_| Error::Other("oneshot dropped".into()))?
    }
}

impl std::fmt::Debug for AgentHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentHandle")
            .field("working_dir", &self.working_dir)
            .finish()
    }
}

/// Spawn a worker that drives main + oneshot runtimes from `provider` and
/// forwards main-turn events into `events`. The caller holds the returned
/// [`AgentHandle`]; the worker exits when the handle is dropped (command
/// channel closes) or `shutdown()` is called.
///
/// On the first `RunTurn`, the worker `ensure_hot`s main and kicks a
/// best-effort oneshot warm-up. Title/reply requests serialize on the oneshot
/// path and rotate after each successful prompt (N=1). Each oneshot Work item
/// is bounded by [`ONESHOT_CALL_BUDGET`].
pub fn spawn_worker<P: Provider + 'static>(
    provider: P,
    working_dir: PathBuf,
    events: mpsc::Sender<AgentEvent>,
    oneshot_model: Option<String>,
) -> AgentHandle {
    spawn_worker_with_oneshot_budget(
        provider,
        working_dir,
        events,
        ONESHOT_CALL_BUDGET,
        oneshot_model,
    )
}

/// Like [`spawn_worker`], but with an explicit oneshot Work budget (tests inject
/// a short budget so hang recovery does not wait the full production budget).
fn spawn_worker_with_oneshot_budget<P: Provider + 'static>(
    provider: P,
    working_dir: PathBuf,
    events: mpsc::Sender<AgentEvent>,
    oneshot_budget: Duration,
    oneshot_model: Option<String>,
) -> AgentHandle {
    let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<AgentCommand>();
    let (oneshot_tx, mut oneshot_rx) = mpsc::unbounded_channel::<OneshotCommand>();
    let cancel = CancelToken::new();
    let pending_choices = PendingUserChoices::shared();
    let handle = AgentHandle {
        cancel: cancel.clone(),
        tx: cmd_tx,
        oneshot_tx: oneshot_tx.clone(),
        working_dir: working_dir.clone(),
        pending_choices: pending_choices.clone(),
    };

    let mut main = provider.open_main_runtime(&working_dir);
    let mut oneshot = provider.open_oneshot_runtime(&working_dir, oneshot_model);

    // Oneshot loop: serializes title + reply; concurrent with the main loop.
    tokio::spawn(async move {
        while let Some(cmd) = oneshot_rx.recv().await {
            match cmd {
                OneshotCommand::Warm => {
                    let _ = oneshot.ensure_hot().await;
                }
                OneshotCommand::Work(req) => {
                    let result = match timeout(oneshot_budget, async {
                        oneshot.ensure_hot().await?;
                        oneshot.prompt(req.kind, req.prompt).await
                    })
                    .await
                    {
                        Ok(inner) => inner,
                        Err(_elapsed) => Err(Error::Timeout(
                            "oneshot call exceeded budget".into(),
                        )),
                    };
                    let ok = result.is_ok();
                    let _ = req.reply.send(result);
                    // N=1: rotate after a successful prompt before the next work item.
                    // Any Err (including timeout) cold-resets oneshot heat so the
                    // serial queue cannot stay wedged behind a dead child.
                    if ok {
                        let _ = oneshot.rotate().await;
                    } else {
                        oneshot.shutdown().await;
                    }
                }
                OneshotCommand::Shutdown => {
                    oneshot.shutdown().await;
                    return;
                }
            }
        }
        oneshot.shutdown().await;
    });

    // Main loop: turns, session id, cancel (via token), shutdown.
    tokio::spawn(async move {
        let mut session_id: Option<String> = None;
        let mut first_turn = true;

        while let Some(cmd) = cmd_rx.recv().await {
            match cmd {
                AgentCommand::RunTurn(mut req) => {
                    cancel.reset();
                    if req.session_id.is_none() {
                        req.session_id = session_id.clone();
                    }

                    if first_turn {
                        first_turn = false;
                        // Best-effort oneshot warm while main starts (or runs).
                        let _ = oneshot_tx.send(OneshotCommand::Warm);
                    }

                    if let Err(e) = main.ensure_hot().await {
                        let send_result = events
                            .send(AgentEvent::Error(e.to_string()))
                            .await
                            .map_err(|_| ());
                        if send_result.is_err() {
                            break;
                        }
                        continue;
                    }

                    let outcome = main
                        .run_turn(
                            req,
                            events.clone(),
                            cancel.clone(),
                            pending_choices.clone(),
                        )
                        .await;
                    let send_result = match outcome {
                        Ok(out) => {
                            let changed = session_id.as_deref() != Some(out.session_id.as_str());
                            session_id = Some(out.session_id.clone());
                            let mut r = Ok(());
                            if changed {
                                r = events
                                    .send(AgentEvent::SessionIdUpdated {
                                        session_id: out.session_id,
                                    })
                                    .await
                                    .map_err(|_| ());
                            }
                            if r.is_ok() {
                                r = events
                                    .send(AgentEvent::TurnComplete)
                                    .await
                                    .map_err(|_| ());
                            }
                            r
                        }
                        Err(Error::Cancelled) => events
                            .send(AgentEvent::TurnComplete)
                            .await
                            .map_err(|_| ()),
                        Err(e) if e.is_session_not_found() => {
                            // Dead resume id — forget it so a retry opens
                            // session/new instead of looping on session/load.
                            session_id = None;
                            events
                                .send(AgentEvent::SessionNotFound)
                                .await
                                .map_err(|_| ())
                        }
                        Err(e) => events
                            .send(AgentEvent::Error(e.to_string()))
                            .await
                            .map_err(|_| ()),
                    };
                    // If the receiver is gone (subscription torn down) there's
                    // no point in continuing to process queued commands.
                    if send_result.is_err() {
                        break;
                    }
                }
                AgentCommand::AnswerUserChoice {
                    correlation_id,
                    answer,
                } => {
                    pending_choices.answer(correlation_id, answer);
                }
                AgentCommand::SetSessionId(sid) => {
                    session_id = Some(sid);
                }
                AgentCommand::ClearSessionId => {
                    session_id = None;
                }
                AgentCommand::Shutdown => {
                    pending_choices.cancel_all();
                    main.shutdown().await;
                    let _ = oneshot_tx.send(OneshotCommand::Shutdown);
                    return;
                }
            }
        }
        // Channel closed without Shutdown — still drop any held children.
        main.shutdown().await;
        let _ = oneshot_tx.send(OneshotCommand::Shutdown);
    });

    handle
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use async_trait::async_trait;
    use tokio::sync::{Notify, mpsc};

    use super::*;
    use crate::provider::{Capabilities, Provider, SlashCommand};
    use crate::request::TurnOutcome;
    use crate::runtime::{MainRuntime, OneshotRuntime};

    /// Shared log of runtime activity for assertions.
    #[derive(Default)]
    struct FakeLog {
        main_ensure_hot: AtomicUsize,
        main_turns: AtomicUsize,
        main_cancelled_turns: AtomicUsize,
        main_shutdown: AtomicUsize,
        oneshot_ensure_hot: AtomicUsize,
        oneshot_prompts: Mutex<Vec<(OneshotKind, String, u64)>>,
        oneshot_sessions: Mutex<Vec<u64>>,
        oneshot_rotate: AtomicUsize,
        oneshot_shutdown: AtomicUsize,
        /// Preferred model id passed into `open_oneshot_runtime`.
        oneshot_preferred: Mutex<Option<String>>,
        /// True while a oneshot prompt body is executing.
        oneshot_in_flight: AtomicBool,
        /// Peak concurrent oneshot prompt executions (should stay ≤ 1).
        oneshot_max_concurrent: AtomicUsize,
        oneshot_concurrent: AtomicUsize,
    }

    impl FakeLog {
        fn prompt_kinds(&self) -> Vec<OneshotKind> {
            self.oneshot_prompts
                .lock()
                .unwrap()
                .iter()
                .map(|(k, _, _)| *k)
                .collect()
        }

        fn session_ids_used(&self) -> Vec<u64> {
            self.oneshot_prompts
                .lock()
                .unwrap()
                .iter()
                .map(|(_, _, sid)| *sid)
                .collect()
        }
    }

    struct FakeProvider {
        log: Arc<FakeLog>,
        /// When set, the first main turn blocks until cancel or this notify.
        hang_first_turn: Option<Arc<Notify>>,
        hang_released: Arc<AtomicBool>,
        /// When true, the first oneshot `prompt` hangs until dropped (budget timeout).
        hang_first_oneshot: bool,
    }

    impl FakeProvider {
        fn new(log: Arc<FakeLog>) -> Self {
            Self {
                log,
                hang_first_turn: None,
                hang_released: Arc::new(AtomicBool::new(false)),
                hang_first_oneshot: false,
            }
        }

        fn with_hang(log: Arc<FakeLog>, hang: Arc<Notify>) -> Self {
            Self {
                log,
                hang_first_turn: Some(hang),
                hang_released: Arc::new(AtomicBool::new(false)),
                hang_first_oneshot: false,
            }
        }

        fn with_hang_first_oneshot(log: Arc<FakeLog>) -> Self {
            Self {
                log,
                hang_first_turn: None,
                hang_released: Arc::new(AtomicBool::new(false)),
                hang_first_oneshot: true,
            }
        }
    }

    struct FakeMainRuntime {
        log: Arc<FakeLog>,
        hang_first_turn: Option<Arc<Notify>>,
        hang_released: Arc<AtomicBool>,
        turn_count: usize,
        hot: bool,
    }

    struct FakeOneshotRuntime {
        log: Arc<FakeLog>,
        hot: bool,
        /// Logical oneshot session id; rotate advances it.
        session_id: u64,
        next_session: u64,
        hang_first_oneshot: bool,
        hung_once: bool,
    }

    /// Restores concurrent bookkeeping if `prompt` is cancelled mid-flight (timeout).
    struct OneshotInFlightGuard<'a> {
        log: &'a FakeLog,
    }

    impl Drop for OneshotInFlightGuard<'_> {
        fn drop(&mut self) {
            self.log.oneshot_concurrent.fetch_sub(1, Ordering::SeqCst);
            self.log.oneshot_in_flight.store(false, Ordering::SeqCst);
        }
    }

    #[async_trait]
    impl MainRuntime for FakeMainRuntime {
        async fn ensure_hot(&mut self) -> Result<(), Error> {
            self.log.main_ensure_hot.fetch_add(1, Ordering::SeqCst);
            self.hot = true;
            Ok(())
        }

        async fn run_turn(
            &mut self,
            req: TurnRequest,
            _events: mpsc::Sender<AgentEvent>,
            cancel: CancelToken,
            _pending_choices: std::sync::Arc<PendingUserChoices>,
        ) -> Result<TurnOutcome, Error> {
            self.turn_count += 1;
            // First turn may hang until cancelled (for cancel/re-warm tests).
            if self.turn_count == 1
                && let Some(hang) = &self.hang_first_turn
            {
                hang.notify_one();
                loop {
                    if cancel.is_cancelled() {
                        self.hot = false;
                        self.log.main_cancelled_turns.fetch_add(1, Ordering::SeqCst);
                        self.hang_released.store(true, Ordering::SeqCst);
                        return Err(Error::Cancelled);
                    }
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
            }
            if cancel.is_cancelled() {
                self.hot = false;
                self.log.main_cancelled_turns.fetch_add(1, Ordering::SeqCst);
                return Err(Error::Cancelled);
            }
            self.log.main_turns.fetch_add(1, Ordering::SeqCst);
            let sid = req
                .session_id
                .clone()
                .unwrap_or_else(|| format!("sess-{}", self.turn_count));
            Ok(TurnOutcome { session_id: sid })
        }

        async fn shutdown(&mut self) {
            self.hot = false;
            self.log.main_shutdown.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[async_trait]
    impl OneshotRuntime for FakeOneshotRuntime {
        async fn ensure_hot(&mut self) -> Result<(), Error> {
            self.log.oneshot_ensure_hot.fetch_add(1, Ordering::SeqCst);
            self.hot = true;
            if self.session_id == 0 {
                self.next_session += 1;
                self.session_id = self.next_session;
            }
            Ok(())
        }

        async fn prompt(&mut self, model_hint: OneshotKind, text: String) -> Result<String, Error> {
            let concurrent = self.log.oneshot_concurrent.fetch_add(1, Ordering::SeqCst) + 1;
            self.log.oneshot_in_flight.store(true, Ordering::SeqCst);
            let _guard = OneshotInFlightGuard { log: &self.log };
            let max = self.log.oneshot_max_concurrent.load(Ordering::SeqCst);
            if concurrent > max {
                self.log
                    .oneshot_max_concurrent
                    .store(concurrent, Ordering::SeqCst);
            }

            if self.hang_first_oneshot && !self.hung_once {
                self.hung_once = true;
                // Hang until the worker's oneshot budget cancels this future.
                loop {
                    tokio::time::sleep(Duration::from_secs(60)).await;
                }
            }

            // Small delay so concurrent callers would overlap if not serialized.
            tokio::time::sleep(Duration::from_millis(30)).await;

            let sid = self.session_id;
            self.log
                .oneshot_prompts
                .lock()
                .unwrap()
                .push((model_hint, text.clone(), sid));
            self.log
                .oneshot_sessions
                .lock()
                .unwrap()
                .push(sid);

            match model_hint {
                OneshotKind::Title => Ok(format!("\"Title for session {sid}.\"")),
                OneshotKind::ReplySuggest => Ok(format!("REPLY: /ds-spec\nREPLY: no thanks ({sid})")),
            }
        }

        async fn rotate(&mut self) -> Result<(), Error> {
            self.log.oneshot_rotate.fetch_add(1, Ordering::SeqCst);
            // Fresh logical session; process stays hot.
            self.next_session += 1;
            self.session_id = self.next_session;
            Ok(())
        }

        async fn shutdown(&mut self) {
            self.hot = false;
            // Cold-reset clears logical session so the next ensure_hot opens fresh.
            self.session_id = 0;
            self.log.oneshot_shutdown.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[async_trait]
    impl Provider for FakeProvider {
        fn id(&self) -> &str {
            "fake-cold"
        }

        fn capabilities(&self) -> Capabilities {
            Capabilities {
                streaming: true,
                tool_use: false,
                resume: true,
                reasoning: false,
                slash_commands: false,
            }
        }

        fn list_commands(&self, _project_root: &Path) -> Vec<SlashCommand> {
            Vec::new()
        }

        fn open_main_runtime(&self, _working_dir: &Path) -> Box<dyn MainRuntime> {
            Box::new(FakeMainRuntime {
                log: self.log.clone(),
                hang_first_turn: self.hang_first_turn.clone(),
                hang_released: self.hang_released.clone(),
                turn_count: 0,
                hot: false,
            })
        }

        fn open_oneshot_runtime(
            &self,
            _working_dir: &Path,
            preferred_model: Option<String>,
        ) -> Box<dyn OneshotRuntime> {
            *self.log.oneshot_preferred.lock().unwrap() = preferred_model;
            Box::new(FakeOneshotRuntime {
                log: self.log.clone(),
                hot: false,
                session_id: 0,
                next_session: 0,
                hang_first_oneshot: self.hang_first_oneshot,
                hung_once: false,
            })
        }

        async fn title_summary(
            &self,
            _req: TitleRequest,
            _working_dir: &Path,
        ) -> Result<String, Error> {
            Err(Error::Other("use handle".into()))
        }

        async fn reply_suggestions(
            &self,
            _req: ReplySuggestionRequest,
            _working_dir: &Path,
        ) -> Result<Vec<String>, Error> {
            Err(Error::Other("use handle".into()))
        }
    }

    async fn drain_until_turn_complete(rx: &mut mpsc::Receiver<AgentEvent>) {
        while let Some(ev) = rx.recv().await {
            if matches!(ev, AgentEvent::TurnComplete) {
                return;
            }
            if matches!(ev, AgentEvent::Error(_)) {
                panic!("unexpected error event: {ev:?}");
            }
        }
        panic!("event channel closed before TurnComplete");
    }

    // @spec harness/warm-runtime Per-chat handle ownership: Title summary is requested through the chat handle
    #[tokio::test]
    async fn title_summary_is_requested_through_the_chat_handle() {
        let log = Arc::new(FakeLog::default());
        let (tx, mut rx) = mpsc::channel(16);
        let handle = spawn_worker(FakeProvider::new(log.clone()), std::env::temp_dir(), tx, None);

        // First turn activates paths (not required for title, but realistic).
        handle.send_prompt("hello".into());
        drain_until_turn_complete(&mut rx).await;

        let title = handle
            .title_summary(TitleRequest::new("implement warm runtimes"))
            .await
            .expect("title through handle");
        assert!(!title.is_empty());
        assert!(!title.contains('"'));
        assert!(log.prompt_kinds().contains(&OneshotKind::Title));
        handle.shutdown();
    }

    // @spec harness/warm-runtime Per-chat handle ownership: Reply suggestions are requested through the chat handle
    #[tokio::test]
    async fn reply_suggestions_are_requested_through_the_chat_handle() {
        let log = Arc::new(FakeLog::default());
        let (tx, mut rx) = mpsc::channel(16);
        let handle = spawn_worker(FakeProvider::new(log.clone()), std::env::temp_dir(), tx, None);

        handle.send_prompt("hello".into());
        drain_until_turn_complete(&mut rx).await;

        let replies = handle
            .reply_suggestions(ReplySuggestionRequest::new(
                "Ready for the next step. Run /ds-spec when you want.",
            ))
            .await
            .expect("replies through handle");
        assert!(!replies.is_empty());
        assert!(log.prompt_kinds().contains(&OneshotKind::ReplySuggest));
        handle.shutdown();
    }

    // @spec harness/warm-runtime Lazy activation: First turn succeeds without a prior pre-warm call
    #[tokio::test]
    async fn first_turn_succeeds_without_a_prior_pre_warm_call() {
        let log = Arc::new(FakeLog::default());
        let (tx, mut rx) = mpsc::channel(16);
        let handle = spawn_worker(FakeProvider::new(log.clone()), std::env::temp_dir(), tx, None);

        // No ensure_hot / pre-warm API call — just send.
        handle.send_prompt("first turn".into());
        drain_until_turn_complete(&mut rx).await;

        assert!(
            log.main_turns.load(Ordering::SeqCst) >= 1,
            "turn should complete"
        );
        assert!(
            log.main_ensure_hot.load(Ordering::SeqCst) >= 1,
            "worker activates main on first turn"
        );
        handle.shutdown();
    }

    // @spec harness/warm-runtime Lazy activation: Oneshot after first send needs no separate pre-warm API
    #[tokio::test]
    async fn oneshot_after_first_send_needs_no_separate_pre_warm_api() {
        let log = Arc::new(FakeLog::default());
        let (tx, mut rx) = mpsc::channel(16);
        let handle = spawn_worker(FakeProvider::new(log.clone()), std::env::temp_dir(), tx, None);

        handle.send_prompt("first".into());
        drain_until_turn_complete(&mut rx).await;

        // No separate pre-warm API — title_summary alone is enough.
        let title = handle
            .title_summary(TitleRequest::new("after first send"))
            .await
            .expect("oneshot after first send");
        assert!(!title.is_empty());
        handle.shutdown();
    }

    // @spec harness/warm-runtime Oneshot serialization and isolation: Title and reply suggestions run one at a time on the oneshot path
    #[tokio::test]
    async fn title_and_reply_suggestions_run_one_at_a_time_on_the_oneshot_path() {
        let log = Arc::new(FakeLog::default());
        let (tx, mut rx) = mpsc::channel(16);
        let handle = spawn_worker(FakeProvider::new(log.clone()), std::env::temp_dir(), tx, None);

        handle.send_prompt("go".into());
        drain_until_turn_complete(&mut rx).await;

        let h1 = handle.clone();
        let h2 = handle.clone();
        let (title_res, reply_res) = tokio::join!(
            h1.title_summary(TitleRequest::new("serialize me")),
            h2.reply_suggestions(ReplySuggestionRequest::new("assistant asks next?")),
        );

        let title = title_res.expect("title");
        let replies = reply_res.expect("replies");
        assert!(!title.is_empty());
        assert!(!replies.is_empty());
        assert_eq!(
            log.oneshot_max_concurrent.load(Ordering::SeqCst),
            1,
            "oneshot path must not run title and reply concurrently"
        );
        let kinds = log.prompt_kinds();
        assert!(kinds.contains(&OneshotKind::Title));
        assert!(kinds.contains(&OneshotKind::ReplySuggest));
        handle.shutdown();
    }

    // @spec harness/warm-runtime Oneshot serialization and isolation: A second oneshot call does not resume the prior oneshot session
    #[tokio::test]
    async fn a_second_oneshot_call_does_not_resume_the_prior_oneshot_session() {
        let log = Arc::new(FakeLog::default());
        let (tx, mut rx) = mpsc::channel(16);
        let handle = spawn_worker(FakeProvider::new(log.clone()), std::env::temp_dir(), tx, None);

        handle.send_prompt("go".into());
        drain_until_turn_complete(&mut rx).await;

        let _ = handle
            .title_summary(TitleRequest::new("first oneshot"))
            .await
            .unwrap();
        let _ = handle
            .title_summary(TitleRequest::new("second oneshot"))
            .await
            .unwrap();

        let sessions = log.session_ids_used();
        assert_eq!(sessions.len(), 2);
        assert_ne!(
            sessions[0], sessions[1],
            "second oneshot must not resume prior oneshot session"
        );
        assert!(
            log.oneshot_rotate.load(Ordering::SeqCst) >= 2,
            "rotate after each successful oneshot (N=1)"
        );
        handle.shutdown();
    }

    // @spec harness/warm-runtime Cancel and re-warm: After cancel, a later turn on the same handle can complete
    #[tokio::test]
    async fn after_cancel_a_later_turn_on_the_same_handle_can_complete() {
        let log = Arc::new(FakeLog::default());
        let hang = Arc::new(Notify::new());
        let (tx, mut rx) = mpsc::channel(16);
        let handle = spawn_worker(
            FakeProvider::with_hang(log.clone(), hang.clone()),
            std::env::temp_dir(),
            tx,
            None,
        );

        handle.send_prompt("long turn".into());
        // Wait until the fake main is inside the cancellable hang.
        hang.notified().await;
        handle.cancel();

        // First turn ends as cancelled → TurnComplete.
        drain_until_turn_complete(&mut rx).await;

        // Later turn on the same handle still completes (re-warm allowed).
        handle.send_prompt("after cancel".into());
        drain_until_turn_complete(&mut rx).await;

        assert!(
            log.main_turns.load(Ordering::SeqCst) >= 1,
            "later turn should complete after cancel"
        );
        assert!(
            log.main_cancelled_turns.load(Ordering::SeqCst) >= 1,
            "first turn should have been cancelled"
        );
        // Oneshot path still usable (cancel does not tear it down).
        let title = handle
            .title_summary(TitleRequest::new("still works"))
            .await
            .expect("oneshot after main cancel");
        assert!(!title.is_empty());
        handle.shutdown();
    }

    // @spec harness/warm-runtime Cold-capable harnesses: A cold-capable harness serves title summary through the handle
    #[tokio::test]
    async fn a_cold_capable_harness_serves_title_summary_through_the_handle() {
        // Fake provider is cold-capable: no process reuse beyond ensure_hot bookkeeping.
        let log = Arc::new(FakeLog::default());
        let (tx, _rx) = mpsc::channel(16);
        let handle = spawn_worker(FakeProvider::new(log.clone()), std::env::temp_dir(), tx, None);

        let title = handle
            .title_summary(TitleRequest::new("cold path title"))
            .await
            .expect("cold harness title");
        assert!(!title.is_empty());
        assert!(
            !title.contains('.') && !title.contains('"'),
            "plain-text title: {title}"
        );
        handle.shutdown();
    }

    /// Short budget for hang-recovery tests (production uses [`ONESHOT_CALL_BUDGET`]).
    const TEST_ONESHOT_BUDGET: Duration = Duration::from_millis(80);

    // @spec harness/warm-runtime Oneshot call budget and recovery: Over-budget oneshot returns an error
    #[tokio::test]
    async fn over_budget_oneshot_returns_an_error() {
        let log = Arc::new(FakeLog::default());
        let (tx, _rx) = mpsc::channel(16);
        let handle = spawn_worker_with_oneshot_budget(
            FakeProvider::with_hang_first_oneshot(log.clone()),
            std::env::temp_dir(),
            tx,
            TEST_ONESHOT_BUDGET,
            None,
        );

        let started = std::time::Instant::now();
        let err = handle
            .title_summary(TitleRequest::new("will hang past budget"))
            .await
            .expect_err("over-budget oneshot must error");
        assert!(
            matches!(err, Error::Timeout(_)),
            "expected Timeout, got {err:?}"
        );
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "must not remain in flight for the full production budget"
        );
        // Cold-reset after timeout.
        assert!(
            log.oneshot_shutdown.load(Ordering::SeqCst) >= 1,
            "timeout path cold-resets oneshot heat"
        );
        handle.shutdown();
    }

    /// Host-resolved oneshot preference is passed into `open_oneshot_runtime`.
    #[tokio::test]
    async fn spawn_worker_opens_oneshot_with_preferred_model() {
        let log = Arc::new(FakeLog::default());
        let (tx, _rx) = mpsc::channel(16);
        let handle = spawn_worker(
            FakeProvider::new(log.clone()),
            std::env::temp_dir(),
            tx,
            Some("haiku".into()),
        );
        // Allow the worker task to open runtimes.
        tokio::task::yield_now().await;
        tokio::time::sleep(Duration::from_millis(20)).await;
        let preferred = log.oneshot_preferred.lock().unwrap().clone();
        assert_eq!(
            preferred.as_deref(),
            Some("haiku"),
            "expected preferred oneshot model passed to open_oneshot_runtime"
        );
        handle.shutdown();
    }

    // @spec harness/warm-runtime Oneshot call budget and recovery: Later oneshot succeeds after prior oneshot failure
    #[tokio::test]
    async fn later_oneshot_succeeds_after_prior_oneshot_failure() {
        let log = Arc::new(FakeLog::default());
        let (tx, _rx) = mpsc::channel(16);
        let handle = spawn_worker_with_oneshot_budget(
            FakeProvider::with_hang_first_oneshot(log.clone()),
            std::env::temp_dir(),
            tx,
            TEST_ONESHOT_BUDGET,
            None,
        );

        let first = handle
            .title_summary(TitleRequest::new("first hangs"))
            .await;
        assert!(
            matches!(first, Err(Error::Timeout(_))),
            "first oneshot should time out: {first:?}"
        );

        // Same handle: subsequent oneshot must complete after cold-reset.
        let title = handle
            .title_summary(TitleRequest::new("second succeeds"))
            .await
            .expect("later oneshot after prior failure");
        assert!(!title.is_empty());
        assert!(
            log.oneshot_shutdown.load(Ordering::SeqCst) >= 1,
            "failure path cold-resets before next work"
        );
        assert!(
            log.prompt_kinds().contains(&OneshotKind::Title),
            "successful second call should have recorded a prompt"
        );
        handle.shutdown();
    }
}
