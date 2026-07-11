//! Agent chat widget — per-message text editors in a scrollable column.

use iced::widget::{
    Space, button, column, container, pick_list, row, rule, scrollable, stack, text,
};
use iced::widget::text::Wrapping;

pub const CHAT_SCROLLABLE_ID: &str = "agent-chat-scroll";
pub const CHAT_INPUT_ID: &str = "agent-chat-input";
/// Cap the auto-growing chat input at this many visual rows; past it the
/// input scrolls internally to keep the caret visible instead of pushing the
/// chat history (and the caret) off the top of the window.
const CHAT_INPUT_MAX_ROWS: usize = 20;
/// Pixels of slack at the bottom edge that still count as "stuck to bottom".
/// Small enough that one wheel notch unsticks the view, large enough to
/// absorb sub-pixel layout rounding during streaming rebuilds.
pub const STICK_TO_BOTTOM_THRESHOLD: f32 = 16.0;
use iced::{Element, Length};

use duckchat::{ModelInfo, ModelRef};

use crate::agent::SlashCommand;
use crate::area::interaction::{self, SelectionContext};
use crate::chat_store::{ChatSession, ContentBlock, Role};
use crate::theme;
use crate::widget::collapsible;
use crate::widget::streaming_indicator;
use crate::widget::text_edit::{self, Block, BlockKind, EditorState};

// ── Messages ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Msg {
    /// Action from the chat input editor.
    InputAction(text_edit::EditorAction),
    SendPressed,
    /// Activate an obvious-chrome action (key or chip click). Payload is the
    /// action send text only (not the hotkey label).
    SendObviousAction(String),
    CancelPressed,
    CompletionAccept,
    CompletionNext,
    CompletionPrev,
    CompletionDismiss,
    /// Action from a per-block chat text editor (index, action).
    ChatAction(usize, text_edit::EditorAction),
    /// Toggle collapse state of a block.
    ToggleCollapse(usize),
    /// Action from the queued-message read-only editor.
    QueueAction(text_edit::EditorAction),
    /// Discard the queued message (from the pill's ✕ button).
    DiscardQueue,
    /// User scrolled the chat transcript. Drives the per-session
    /// `stick_to_bottom` flag — true while the viewport is within
    /// `STICK_TO_BOTTOM_THRESHOLD` pixels of the bottom.
    ChatScrolled(scrollable::Viewport),
    /// User picked a model from the meta-row selector.
    ModelSelected(ModelChoice),
    /// Cycle empty-input next actions (`+1` Tab, `-1` Shift-Tab).
    CycleNextAction(i8),
    /// Empty Cmd-Enter: send the armed oneshot suggestion when ready.
    SendOneshotSuggestion,
    /// Layout measure of the chat scrollable (viewport + content heights).
    /// Used to recompute the bottom-pin pad even when content fits the
    /// viewport and iced suppresses `on_scroll` notifications.
    ChromeLayout { viewport_h: f32, content_h: f32 },
}

// ── Model picker ─────────────────────────────────────────────────────────────

/// One entry in the meta-row model `pick_list`. `id` is the `--model` value to
/// pin and `harness` the backend that owns it (`None`/`None` = "use project
/// default"). Equality is on `(harness, id)` so a picked model resolves to the
/// right harness even when two backends share a bare model id — while the
/// selected entry can still carry a richer label (e.g. the resolved model in
/// parens) and match its plain option in the dropdown.
#[derive(Debug, Clone)]
pub struct ModelChoice {
    /// The harness owning this model, e.g. `"claude-code"` | `"grok"`. `None`
    /// on the "use default" sentinel, which pins no specific model.
    pub harness: Option<String>,
    pub id: Option<String>,
    /// Menu / list label — harness-prefixed so multi-backend choices stay grouped.
    pub label: String,
    /// Short name for the closed control (model display only, no harness prefix).
    pub closed_label: String,
}

impl ModelChoice {
    /// The persisted model reference this choice selects, or `None` for the
    /// "use default" sentinel (which carries neither harness nor id).
    pub fn to_ref(&self) -> Option<ModelRef> {
        match (&self.harness, &self.id) {
            (Some(harness), Some(id)) => Some(ModelRef::new(harness.clone(), id.clone())),
            _ => None,
        }
    }
}

impl PartialEq for ModelChoice {
    fn eq(&self, other: &Self) -> bool {
        self.harness == other.harness && self.id == other.id
    }
}

impl Eq for ModelChoice {}

impl std::fmt::Display for ModelChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

/// Picker options for a chat's model selector. The first entry (`id: None`)
/// is the "use project default" sentinel; `project_default` lets its label
/// name the model that default currently resolves to.
pub fn chat_model_choices() -> Vec<ModelChoice> {
    // No "use default" sentinel: the chat selector always shows the concrete
    // model the next turn will run (the resolved cascade), never the word
    // "Default". `selected_model_choice` is fed that effective model.
    model_entries()
}

/// Picker options for the project-level default selector in settings. The
/// first entry (`id: None`) means "no default — let the CLI pick".
pub fn project_model_choices() -> Vec<ModelChoice> {
    let mut out = vec![ModelChoice {
        harness: None,
        id: None,
        label: "No default".to_string(),
        closed_label: "No default".to_string(),
    }];
    out.extend(model_entries());
    out
}

fn model_entries() -> Vec<ModelChoice> {
    group_choices(crate::agent::available_models())
}

/// Turn the aggregated model list into picker entries grouped by harness. Each
/// harness's models are kept contiguous (in first-seen order) and every label
/// is prefixed with its harness so the flat `pick_list` reads as harness
/// sections rather than an undifferentiated list.
fn group_choices(models: Vec<ModelInfo>) -> Vec<ModelChoice> {
    let mut harness_order: Vec<String> = Vec::new();
    for m in &models {
        if !harness_order.contains(&m.harness) {
            harness_order.push(m.harness.clone());
        }
    }
    let mut out = Vec::with_capacity(models.len());
    for harness in &harness_order {
        for m in models.iter().filter(|m| &m.harness == harness) {
            out.push(ModelChoice {
                harness: Some(m.harness.clone()),
                id: Some(m.id.clone()),
                label: format!("{} · {}", harness_display(&m.harness), m.display),
                closed_label: m.display.clone(),
            });
        }
    }
    out
}

/// Human-friendly name for a harness id, used to label and group picker
/// entries. Unknown ids fall back to the raw id.
fn harness_display(harness: &str) -> &str {
    match harness {
        "claude-code" => "Claude Code",
        "grok" => "Grok",
        other => other,
    }
}

/// Resolve which option is selected for a pinned model reference. `None`
/// selects the first (sentinel) entry. A ref not in the offered list (e.g. a
/// full model name, or a harness that dropped out) yields a synthetic choice so
/// the picker still shows it rather than silently falling back to the sentinel.
pub fn selected_model_choice(choices: &[ModelChoice], selected: Option<&ModelRef>) -> ModelChoice {
    match selected {
        None => choices.first().cloned().unwrap_or(ModelChoice {
            harness: None,
            id: None,
            label: "Default".to_string(),
            closed_label: "Default".to_string(),
        }),
        Some(model_ref) => choices
            .iter()
            .find(|c| {
                c.harness.as_deref() == Some(model_ref.harness.as_str())
                    && c.id.as_deref() == Some(model_ref.model.as_str())
            })
            .cloned()
            .unwrap_or(ModelChoice {
                harness: Some(model_ref.harness.clone()),
                id: Some(model_ref.model.clone()),
                label: format!(
                    "{} · {}",
                    harness_display(&model_ref.harness),
                    model_ref.model
                ),
                closed_label: model_ref.model.clone(),
            }),
    }
}

/// The context window of a specific model, looked up by harness + id from the
/// aggregated model list. `None` when the model is unknown or its harness
/// reports no window — the usage meter then shows no fill.
pub fn model_context_window(model: &ModelRef) -> Option<usize> {
    crate::agent::available_models()
        .into_iter()
        .find(|m| m.harness == model.harness && m.id == model.model)
        .and_then(|m| m.context_window)
}

/// Fraction of the selected model's context window consumed by `tokens`. A
/// `None` (or zero) window yields `None`: the usage meter shows no fill rather
/// than computing against a wrong or assumed window.
pub fn context_fill(tokens: usize, window: Option<usize>) -> Option<f32> {
    match window {
        Some(w) if w > 0 => Some(tokens as f32 / w as f32),
        _ => None,
    }
}

/// Fill fraction at which the usage readout expands from percentage-only to
/// full `used / max (%)`. Matches the existing warning color band.
pub const USAGE_HOT_FILL: f32 = 0.75;

/// Whether the composer footer shows the resend-history hint. True only when
/// the next send would open fresh *and* there is transcript to resend.
pub fn show_resend_history_hint(will_resume: bool, has_messages: bool) -> bool {
    !will_resume && has_messages
}

/// Progressive context-usage string for a **known** window. Cool fill (< 75%)
/// is percentage only; hot fill (≥ 75%) includes used, max, and percentage.
/// Callers that lack a window should not use this (model-picker owns "no fill").
pub fn format_usage_readout(tokens: usize, window: usize) -> String {
    let fill = tokens as f32 / window as f32;
    let pct = (fill * 100.0) as usize;
    if fill < USAGE_HOT_FILL {
        format!("{pct}%")
    } else {
        format!(
            "{} / {} ({}%)",
            format_number(tokens),
            format_number(window),
            pct
        )
    }
}

// ── Status bar info ────────────────────────────────────────────────────────

/// Data for the status bar below the chat input.
pub struct StatusInfo {
    pub is_streaming: bool,
    /// 0 = no esc pressed, 1 = one esc pressed (waiting for second).
    pub esc_count: u8,
    /// Picker options — one per provider model, grouped by harness.
    pub model_choices: Vec<ModelChoice>,
    /// The currently-selected picker entry (matched by `(harness, id)`).
    pub selected_model: ModelChoice,
    /// Whether the next turn resumes the agent-side session. Combined with
    /// transcript emptiness via `show_resend_history_hint` for the meta-row
    /// resend indicator.
    pub will_resume: bool,
    pub context_tokens: usize,
    /// The selected model's context window. `None` when the model reports no
    /// window — the meter then shows the token count with no fill.
    pub context_max: Option<usize>,
}

// ── Completion state ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct CompletionState {
    pub visible: bool,
    pub selected: usize,
}

// ── Transcript segments ────────────────────────────────────────────────────

/// One contiguous run of the calm transcript: user/system prose, thinking,
/// answer, or a grouped activity of tools.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TranscriptSeg {
    User {
        lines: Vec<String>,
        /// Synthetic first-turn AGENTS.md / orientation inject. Starts
        /// collapsed so scroll-to-top lands on the real first user message.
        is_priming: bool,
    },
    System {
        lines: Vec<String>,
    },
    Thinking {
        lines: Vec<String>,
        /// True while this segment is still open in the turn (streaming and
        /// no following Answer yet) — not merely "still receiving deltas".
        live: bool,
    },
    Answer {
        lines: Vec<String>,
        live: bool,
    },
    Activity {
        tools: Vec<ToolRow>,
        /// True while the activity group is still open in the turn.
        live: bool,
    },
}

/// One tool call inside an [`TranscriptSeg::Activity`] group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolRow {
    pub id: String,
    /// Human-readable tool summary (`format_tool_summary`), or the tool name
    /// alone for orphan results.
    pub summary: String,
    /// Truncated result output; empty while still running.
    pub output_lines: Vec<String>,
    pub status: ToolRowStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolRowStatus {
    Running,
    Done,
    /// Reserved for later error-shaped output detection.
    #[allow(dead_code)]
    Error,
}

/// Build calm transcript segments from committed messages and live stream
/// buffers. Contiguous same-kind assistant content coalesces; tools pair by
/// call id within an activity run.
pub fn build_transcript_segments(session: &ChatSession) -> Vec<TranscriptSeg> {
    use std::collections::HashMap;

    let mut segs: Vec<TranscriptSeg> = Vec::new();
    // Within the open Activity: row order + id → index for pairing.
    let mut activity_index: HashMap<String, usize> = HashMap::new();

    for msg in &session.messages {
        for cb in &msg.content {
            match (msg.role, cb) {
                (Role::User, ContentBlock::Text(t)) => {
                    activity_index.clear();
                    segs.push(TranscriptSeg::User {
                        lines: text_lines(t),
                        is_priming: msg.is_priming,
                    });
                }
                (Role::System, ContentBlock::Text(t)) => {
                    activity_index.clear();
                    segs.push(TranscriptSeg::System {
                        lines: text_lines(t),
                    });
                }
                (Role::Assistant, ContentBlock::Reasoning(t)) => {
                    activity_index.clear();
                    append_thinking(&mut segs, t, false);
                }
                (Role::Assistant, ContentBlock::Text(t)) => {
                    activity_index.clear();
                    append_answer(&mut segs, t, false);
                }
                (
                    Role::Assistant,
                    ContentBlock::ToolUse { id, name, input },
                ) => {
                    ensure_activity(&mut segs, &mut activity_index);
                    let tools = activity_tools_mut(&mut segs);
                    if let Some(&idx) = activity_index.get(id) {
                        // Duplicate id: refresh summary, leave status/output.
                        tools[idx].summary = format_tool_summary(name, input);
                    } else {
                        let idx = tools.len();
                        activity_index.insert(id.clone(), idx);
                        tools.push(ToolRow {
                            id: id.clone(),
                            summary: format_tool_summary(name, input),
                            output_lines: Vec::new(),
                            status: ToolRowStatus::Running,
                        });
                    }
                }
                (
                    Role::Assistant,
                    ContentBlock::ToolResult { id, name, output },
                ) => {
                    ensure_activity(&mut segs, &mut activity_index);
                    let tools = activity_tools_mut(&mut segs);
                    if let Some(&idx) = activity_index.get(id) {
                        tools[idx].output_lines = truncate_output(output);
                        tools[idx].status = ToolRowStatus::Done;
                    } else {
                        // Orphan result: named done row from the tool name,
                        // never a bare "✓ done" placeholder.
                        let idx = tools.len();
                        activity_index.insert(id.clone(), idx);
                        tools.push(ToolRow {
                            id: id.clone(),
                            summary: name.clone(),
                            output_lines: truncate_output(output),
                            status: ToolRowStatus::Done,
                        });
                    }
                }
                // Non-text user/system content (e.g. tools) is not expected
                // on those roles — skip rather than invent a segment.
                (Role::User | Role::System, _) => {}
            }
        }
    }

    // Live stream buffers: append to or open Thinking / Answer segments.
    if session.is_streaming {
        if !session.pending_reasoning.is_empty() {
            activity_index.clear();
            append_thinking(&mut segs, &session.pending_reasoning, true);
        }
        if !session.pending_text.is_empty() {
            activity_index.clear();
            append_answer(&mut segs, &session.pending_text, true);
        }
    }

    // Settle tool status and turn-open live flags.
    //
    // `live` means "still open in the turn" — not "still receiving deltas of
    // this kind". Committed reasoning is built with live=false above; while
    // streaming and no following Answer, Thinking stays open so collapse
    // policy does not snap it shut when tools start.
    let streaming = session.is_streaming;
    let answer_after: Vec<bool> = (0..segs.len())
        .map(|i| {
            segs[i + 1..]
                .iter()
                .any(|s| matches!(s, TranscriptSeg::Answer { .. }))
        })
        .collect();
    for (i, seg) in segs.iter_mut().enumerate() {
        match seg {
            TranscriptSeg::Activity { tools, live } => {
                if !streaming {
                    for row in tools.iter_mut() {
                        if row.status == ToolRowStatus::Running {
                            row.status = ToolRowStatus::Done;
                        }
                    }
                    *live = false;
                } else {
                    *live = !answer_after[i]
                        || tools.iter().any(|t| t.status == ToolRowStatus::Running);
                }
            }
            TranscriptSeg::Thinking { live, .. } => {
                if !streaming {
                    *live = false;
                } else {
                    // Open until a following Answer appears or the turn ends.
                    *live = !answer_after[i];
                }
            }
            _ => {}
        }
    }

    segs
}

fn text_lines(t: &str) -> Vec<String> {
    t.lines().map(String::from).collect()
}

fn append_thinking(segs: &mut Vec<TranscriptSeg>, text: &str, live: bool) {
    let mut lines = text_lines(text);
    if let Some(TranscriptSeg::Thinking {
        lines: existing,
        live: existing_live,
    }) = segs.last_mut()
    {
        existing.append(&mut lines);
        *existing_live = *existing_live || live;
    } else {
        segs.push(TranscriptSeg::Thinking { lines, live });
    }
}

fn append_answer(segs: &mut Vec<TranscriptSeg>, text: &str, live: bool) {
    let mut lines = text_lines(text);
    if let Some(TranscriptSeg::Answer {
        lines: existing,
        live: existing_live,
    }) = segs.last_mut()
    {
        existing.append(&mut lines);
        *existing_live = *existing_live || live;
    } else {
        segs.push(TranscriptSeg::Answer { lines, live });
    }
}

fn ensure_activity(
    segs: &mut Vec<TranscriptSeg>,
    activity_index: &mut std::collections::HashMap<String, usize>,
) {
    if !matches!(segs.last(), Some(TranscriptSeg::Activity { .. })) {
        activity_index.clear();
        segs.push(TranscriptSeg::Activity {
            tools: Vec::new(),
            live: false,
        });
    }
}

fn activity_tools_mut(segs: &mut [TranscriptSeg]) -> &mut Vec<ToolRow> {
    match segs.last_mut() {
        Some(TranscriptSeg::Activity { tools, .. }) => tools,
        _ => unreachable!("ensure_activity must open an Activity first"),
    }
}

// ── Collapse policy ────────────────────────────────────────────────────────

/// Per-segment collapse flag plus whether the user has manually toggled it.
/// Index-aligned with the transcript segment list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CollapseState {
    pub collapsed: bool,
    /// Once true, auto-collapse must not force this segment shut again.
    pub user_set: bool,
}

/// Seconds after a manual expand of the priming Setup block before it
/// auto-hides again. Mid of the 10–20s product range.
pub const PRIMING_RECOLLAPSE_SECS: u64 = 15;

/// First-sight default: live Thinking/Activity expanded; settled collapsed.
/// Priming User starts collapsed. Other User / Answer / System stay open.
fn first_sight_collapsed(seg: &TranscriptSeg) -> bool {
    match seg {
        TranscriptSeg::Thinking { live, .. } | TranscriptSeg::Activity { live, .. } => !live,
        TranscriptSeg::User { is_priming: true, .. } => true,
        TranscriptSeg::User { is_priming: false, .. }
        | TranscriptSeg::System { .. }
        | TranscriptSeg::Answer { .. } => false,
    }
}

fn has_following_answer(segs: &[TranscriptSeg], idx: usize) -> bool {
    segs[idx + 1..]
        .iter()
        .any(|s| matches!(s, TranscriptSeg::Answer { .. }))
}

/// Sync collapse state with the current segment list.
///
/// - Resizes to match `segs` (truncates if shorter; appends first-sight defaults).
/// - Auto-collapses untoggled Thinking when a following Answer appears or the
///   segment is no longer live (turn settled — see Thinking `live` fixup in
///   [`build_transcript_segments`]).
/// - Auto-collapses untoggled Activity when a following Answer appears or the
///   turn settles (`live == false`).
/// - Keeps untoggled priming User collapsed (first-sight and on rebuild).
/// - Leaves `user_set` segments alone for auto-collapse (priming re-hide after
///   expand is a separate timer, not this sync path).
///
/// Thinking `live` means open-in-turn (streaming, no following Answer), not
/// "still receiving ReasoningDelta", so tool phases keep Thinking expanded.
pub fn sync_collapse_states(states: &mut Vec<CollapseState>, segs: &[TranscriptSeg]) {
    if states.len() > segs.len() {
        states.truncate(segs.len());
    }
    while states.len() < segs.len() {
        let i = states.len();
        states.push(CollapseState {
            collapsed: first_sight_collapsed(&segs[i]),
            user_set: false,
        });
    }

    for (i, seg) in segs.iter().enumerate() {
        if states[i].user_set {
            continue;
        }
        match seg {
            TranscriptSeg::Thinking { live, .. } | TranscriptSeg::Activity { live, .. } => {
                // Settle when Answer follows or the segment is no longer
                // open-in-turn (`!live`). For Thinking, `live` is fixed up so
                // committed reasoning during a tool phase stays open.
                if has_following_answer(segs, i) || !*live {
                    states[i].collapsed = true;
                }
            }
            TranscriptSeg::User { is_priming: true, .. } => {
                // Stay folded until the user clicks to inspect.
                states[i].collapsed = true;
            }
            TranscriptSeg::User { is_priming: false, .. }
            | TranscriptSeg::System { .. }
            | TranscriptSeg::Answer { .. } => {
                states[i].collapsed = false;
            }
        }
    }
}

/// User toggle: flip collapsed and mark as manually set so auto-collapse
/// will not override this segment again.
pub fn toggle_collapse(states: &mut [CollapseState], idx: usize) {
    if let Some(state) = states.get_mut(idx) {
        state.collapsed = !state.collapsed;
        state.user_set = true;
    }
}

/// Collapsed label for the synthetic priming user message.
pub fn priming_collapsed_label(lines: &[String]) -> String {
    let n = lines.len();
    if n == 1 {
        "Setup · 1 line".to_string()
    } else {
        format!("Setup · {n} lines")
    }
}

/// Force-collapse a priming segment after the expand timer. Callers gate on
/// expand generation so stale timers no-op.
pub fn recollapse_priming(states: &mut [CollapseState], idx: usize) {
    if let Some(state) = states.get_mut(idx) {
        state.collapsed = true;
        // Keep `user_set` so sync does not fight a later re-expand path that
        // also marks user_set; the timed re-hide is intentional UX.
    }
}

// ── Segment presentation helpers ───────────────────────────────────────────

/// Collapsed Thinking label: line count only (no duration).
///
/// Examples: `"Thinking · 1 line"`, `"Thinking · 12 lines"`.
pub fn thinking_collapsed_label(lines: &[String]) -> String {
    let n = lines.len();
    if n == 1 {
        "Thinking · 1 line".to_string()
    } else {
        format!("Thinking · {n} lines")
    }
}

/// Collapsed Activity summary: tool count plus sample names from the rows.
///
/// Example: `"4 tools · Read, Shell, Grep"`.
pub fn activity_collapsed_label(tools: &[ToolRow]) -> String {
    let n = tools.len();
    let count = if n == 1 {
        "1 tool".to_string()
    } else {
        format!("{n} tools")
    };
    const SAMPLE: usize = 3;
    let names: Vec<&str> = tools
        .iter()
        .take(SAMPLE)
        .map(|t| tool_display_name(&t.summary))
        .collect();
    if names.is_empty() {
        count
    } else {
        format!("{count} · {}", names.join(", "))
    }
}

/// Humanized tool verb from a summary line (`"Read · path"` → `"Read"`).
fn tool_display_name(summary: &str) -> &str {
    summary.split(" · ").next().unwrap_or(summary).trim()
}

/// Quiet row for an expanded Activity group. Status + summary on the row;
/// truncated output sits under it. Group expand only — no per-tool expand state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityRowView {
    pub status: ToolRowStatus,
    pub status_glyph: &'static str,
    pub summary: String,
    /// Truncated output lines under the row; empty while running or empty result.
    pub output_lines: Vec<String>,
}

/// Shape expanded Activity presentation as one quiet row per tool.
pub fn expanded_activity_rows(tools: &[ToolRow]) -> Vec<ActivityRowView> {
    tools
        .iter()
        .map(|t| ActivityRowView {
            status: t.status,
            status_glyph: tool_status_glyph(t.status),
            summary: t.summary.clone(),
            output_lines: t.output_lines.clone(),
        })
        .collect()
}

pub fn tool_status_glyph(status: ToolRowStatus) -> &'static str {
    match status {
        ToolRowStatus::Running => "●",
        ToolRowStatus::Done => "✓",
        ToolRowStatus::Error => "✗",
    }
}

// ── Build blocks from session ──────────────────────────────────────────────

/// Map transcript segments 1:1 into editor blocks (index-aligned with
/// [`sync_collapse_states`]).
///
/// Contiguous tools form one Activity block; reasoning becomes Reasoning
/// (Thinking); orphan results are named done rows inside Activity — never a
/// bare "✓ done" block. Call after `build_transcript_segments`.
pub fn blocks_from_segments(segs: &[TranscriptSeg]) -> Vec<Block> {
    segs.iter()
        .map(|seg| match seg {
            TranscriptSeg::User {
                lines,
                is_priming,
            } => Block {
                kind: BlockKind::User,
                label: if *is_priming {
                    "Setup".to_string()
                } else {
                    "User".to_string()
                },
                lines: lines.clone(),
                is_priming: *is_priming,
            },
            TranscriptSeg::System { lines } => Block {
                kind: BlockKind::System,
                label: "System".to_string(),
                lines: lines.clone(),
                is_priming: false,
            },
            TranscriptSeg::Thinking { lines, live } => Block {
                kind: BlockKind::Reasoning,
                label: if *live {
                    "Thinking ···".to_string()
                } else {
                    "Thinking".to_string()
                },
                lines: lines.clone(),
                is_priming: false,
            },
            TranscriptSeg::Answer { lines, live } => Block {
                kind: BlockKind::Assistant,
                label: if *live {
                    "Assistant ···".to_string()
                } else {
                    "Assistant".to_string()
                },
                lines: lines.clone(),
                is_priming: false,
            },
            TranscriptSeg::Activity { tools, .. } => Block {
                kind: BlockKind::Activity,
                label: activity_collapsed_label(tools),
                lines: activity_body_lines(tools),
                is_priming: false,
            },
        })
        .collect()
}

/// Quiet-row dump for an expanded Activity body (status + summary + indented
/// truncated output). Group-level expand only — no per-tool expand state.
fn activity_body_lines(tools: &[ToolRow]) -> Vec<String> {
    let mut lines = Vec::new();
    for row in expanded_activity_rows(tools) {
        lines.push(format!("{} {}", row.status_glyph, row.summary));
        for out in &row.output_lines {
            lines.push(format!("  {out}"));
        }
    }
    lines
}

/// Truncate tool output to a reasonable number of lines, filtering
/// non-printable characters that cause rendering artifacts.
fn truncate_output(output: &str) -> Vec<String> {
    const MAX_LINES: usize = 10;
    let cleaned = strip_ansi_escapes(output);
    let cleaned = strip_tool_wrapper_tags(&cleaned);
    let all_lines: Vec<String> = cleaned
        .lines()
        .map(sanitize_line)
        .map(|s| s.trim_end().to_string())
        .collect();
    let mut lines = if all_lines.len() > MAX_LINES {
        let mut truncated = all_lines[..MAX_LINES].to_vec();
        truncated.push(format!("… ({} more lines)", all_lines.len() - MAX_LINES));
        truncated
    } else {
        all_lines
    };
    while lines.last().is_some_and(|s| s.is_empty()) {
        lines.pop();
    }
    lines
}

/// Remove ANSI CSI escape sequences (e.g. `\x1B[32m`). Parses the sequence
/// greedily through its final byte so the parameter bytes don't leak through.
fn strip_ansi_escapes(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\x1B' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('[') => {
                for next in chars.by_ref() {
                    let n = next as u32;
                    if (0x40..=0x7E).contains(&n) {
                        break;
                    }
                }
            }
            Some(']') => {
                // OSC: terminate on BEL (0x07) or ESC \\.
                let mut prev_esc = false;
                for next in chars.by_ref() {
                    if next == '\x07' {
                        break;
                    }
                    if prev_esc && next == '\\' {
                        break;
                    }
                    prev_esc = next == '\x1B';
                }
            }
            Some(_) | None => {}
        }
    }
    out
}

/// Strip `<tool_use_error>` / `<tool_use_result>` wrapper tags that some
/// agent backends emit around tool output.
fn strip_tool_wrapper_tags(input: &str) -> String {
    input
        .replace("<tool_use_error>", "")
        .replace("</tool_use_error>", "")
        .replace("<tool_use_result>", "")
        .replace("</tool_use_result>", "")
}

/// Replace remaining non-printable / non-standard-whitespace characters with a
/// space to avoid rendering rectangles in the monospace font.
fn sanitize_line(line: &str) -> String {
    line.chars()
        .map(|c| if c == '\t' || c.is_control() { ' ' } else { c })
        .collect()
}

/// Produce a short human-readable summary of a tool call.
///
/// Shape: `Verb · detail` (or just `Verb`). Known Claude/Grok tools share a
/// calm display verb; unknown names are humanized. Never dumps raw JSON.
///
/// Examples: `Read · agent_chat.rs`, `Shell · cargo test -p duckboard`.
fn format_tool_summary(name: &str, input: &str) -> String {
    let verb = known_tool_verb(name)
        .map(str::to_string)
        .unwrap_or_else(|| humanize_tool_name(name));
    match tool_detail(input) {
        Some(detail) if !detail.is_empty() => format!("{verb} · {detail}"),
        _ => verb,
    }
}

/// Map known Claude / Grok tool names to a short display verb (case-insensitive).
/// Returns `None` for unmapped names (use [`humanize_tool_name`]).
fn known_tool_verb(name: &str) -> Option<&'static str> {
    let key = normalize_tool_key(name);
    match key.as_str() {
        // Shell
        "bash" | "shell" | "run_terminal_command" | "run_terminal" => Some("Shell"),
        // File read
        "read" | "read_file" => Some("Read"),
        // File write
        "write" | "write_file" => Some("Write"),
        // Edit / replace
        "edit" | "search_replace" | "multi_edit" | "str_replace" | "strreplace" => Some("Edit"),
        // Search
        "grep" | "rg" => Some("Grep"),
        // Glob / list
        "glob" => Some("Glob"),
        "ls" | "list" | "list_dir" => Some("List"),
        // Web
        "web_search" | "websearch" => Some("Search"),
        "web_fetch" | "webfetch" | "open_page" | "open_page_with_find" | "web_fetch_url" => {
            Some("Fetch")
        }
        // Misc agent tools
        "todo_write" | "todowrite" => Some("Todo"),
        "task" | "spawn_subagent" => Some("Task"),
        "image_gen" | "image_edit" => Some("Image"),
        _ => None,
    }
}

/// Normalize a tool name for alias matching: `WebSearch` → `web_search`,
/// `run-terminal-command` → `run_terminal_command`.
fn normalize_tool_key(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    for (i, c) in name.trim().chars().enumerate() {
        if c == '-' || c == ' ' {
            if !out.ends_with('_') {
                out.push('_');
            }
        } else if c.is_uppercase() {
            if i > 0 && !out.ends_with('_') {
                out.push('_');
            }
            for lower in c.to_lowercase() {
                out.push(lower);
            }
        } else {
            out.push(c);
        }
    }
    while out.contains("__") {
        out = out.replace("__", "_");
    }
    out.trim_matches('_').to_string()
}

/// Humanize an unknown tool name for display: `some_obscure_tool` →
/// `Some obscure tool`, `camelCase` → `Camel case`.
fn humanize_tool_name(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return "Tool".to_string();
    }
    let key = normalize_tool_key(trimmed);
    let words: Vec<&str> = key.split('_').filter(|w| !w.is_empty()).collect();
    if words.is_empty() {
        return "Tool".to_string();
    }
    let mut parts = Vec::with_capacity(words.len());
    for (i, w) in words.iter().enumerate() {
        if i == 0 {
            // Title-case the first word.
            let mut chars = w.chars();
            let first = chars
                .next()
                .map(|c| c.to_uppercase().to_string())
                .unwrap_or_default();
            parts.push(format!("{first}{}", chars.as_str()));
        } else {
            parts.push(w.to_string());
        }
    }
    parts.join(" ")
}

/// Extract a single-line detail from tool input JSON for the summary row.
/// Prefer path, command, pattern/query; never multi-line bodies or full JSON.
fn tool_detail(input: &str) -> Option<String> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }
    let Ok(serde_json::Value::Object(map)) = serde_json::from_str(input) else {
        // Non-JSON input: one short line if it's already a simple string.
        let one = input.lines().next().unwrap_or(input).trim();
        if one.is_empty() || one.starts_with('{') {
            return None;
        }
        return Some(truncate_chars(one, 50).to_string());
    };

    // Path-like fields (shorten to last components).
    for key in [
        "file_path",
        "path",
        "target_file",
        "file",
        "filename",
        "target_directory",
    ] {
        if let Some(p) = map.get(key).and_then(|v| v.as_str()) {
            let p = p.trim();
            if !p.is_empty() {
                return Some(shorten_path(p));
            }
        }
    }

    // Shell command — first line only.
    if let Some(cmd) = map.get("command").and_then(|v| v.as_str()) {
        let one = cmd.lines().next().unwrap_or(cmd).trim();
        if !one.is_empty() {
            return Some(truncate_chars(one, 50).to_string());
        }
    }

    // Search pattern / query — quoted.
    for key in ["pattern", "query"] {
        if let Some(s) = map.get(key).and_then(|v| v.as_str()) {
            let s = s.trim();
            if !s.is_empty() {
                let t = truncate_chars(s, 40);
                return Some(format!("\"{t}\""));
            }
        }
    }

    // URL (web fetch / open).
    if let Some(url) = map.get("url").and_then(|v| v.as_str()) {
        let url = url.trim();
        if !url.is_empty() {
            return Some(truncate_chars(url, 48).to_string());
        }
    }

    // Fallback: first short single-line string field that isn't a bulky body.
    const SKIP: &[&str] = &[
        "contents",
        "content",
        "body",
        "old_string",
        "new_string",
        "output",
        "prompt",
        "text",
        "code",
        "diff",
    ];
    for (key, value) in &map {
        if SKIP.iter().any(|s| s.eq_ignore_ascii_case(key)) {
            continue;
        }
        let Some(s) = value.as_str() else {
            continue;
        };
        let s = s.trim();
        if s.is_empty() || s.contains('\n') {
            continue;
        }
        if s.chars().count() > 80 {
            continue;
        }
        return Some(truncate_chars(s, 40).to_string());
    }

    None
}

/// Shorten a path to at most the last three components.
fn shorten_path(p: &str) -> String {
    let short: String = p
        .rsplit('/')
        .take(3)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("/");
    if short.chars().count() > 48 {
        truncate_chars(&short, 48).to_string()
    } else {
        short
    }
}

/// Truncate a string to at most `max` characters, on char boundaries.
///
/// Slicing with a byte index (`&s[..n]`) panics when the index falls inside a
/// multibyte UTF-8 character, so we count by `char` instead. Returns the whole
/// string when it is already short enough.
fn truncate_chars(s: &str, max: usize) -> &str {
    match s.char_indices().nth(max) {
        Some((byte_idx, _)) => &s[..byte_idx],
        None => s,
    }
}

// ── View ────────────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub fn view<'a>(
    session: &'a ChatSession,
    blocks: &'a [Block],
    editors: &'a [EditorState],
    collapse: &'a [CollapseState],
    input_value: &'a EditorState,
    queue_editor: Option<&'a EditorState>,
    commands: &'a [SlashCommand],
    completion: &CompletionState,
    status: StatusInfo,
    next_actions: &'a [crate::meta_card::NextAction],
    next_action_idx: usize,
    // Under-input oneshot suggestions (never next-card actions).
    oneshot_prompts: Vec<String>,
    // True while the reply-suggestion oneshot is outstanding.
    default_prompts_pending: bool,
    // Multi-option obvious chrome (send form derived in view).
    obvious_chrome: &'a crate::obvious_bubble::ObviousChrome,
    // Spacer above chrome when history is shorter than the viewport.
    chrome_top_pad: f32,
    pinned_selections: &'a [SelectionContext],
    tentative_selection: Option<&'a SelectionContext>,
    block_highlights: Vec<(
        Vec<text_edit::HighlightRange>,
        Option<text_edit::HighlightRange>,
    )>,
) -> Element<'a, Msg> {
    // Chat content — scrollable column of full-width sections.
    let mut chat_col = column![]
        .spacing(theme::SPACING_XS)
        .padding([theme::SPACING_SM, 0.0]);

    let mut block_highlights = block_highlights;
    for (i, block) in blocks.iter().enumerate() {
        let is_collapsed = collapse.get(i).map(|s| s.collapsed).unwrap_or(false);
        let (ranges, current) = if i < block_highlights.len() {
            std::mem::take(&mut block_highlights[i])
        } else {
            (Vec::new(), None)
        };
        let block_el = view_block(i, block, editors.get(i), is_collapsed, ranges, current);
        // Tag each block with a stable widget id so `widget::find` can read
        // the laid-out bounds during an Operation pass and scroll the
        // matching block to the top of the viewport — bypasses all the
        // per-kind padding / wrap / collapse pixel math.
        let tagged = container(block_el)
            .id(crate::widget::find::chat_block_widget_id(i))
            .width(Length::Fill);
        chat_col = chat_col.push(tagged);
    }

    // Streaming indicator: animated pulsing dots + inline cancel hint at
    // the bottom of the transcript, visible only while the agent is
    // producing a response. The left padding (`SPACING_MD + SPACING_SM`)
    // mirrors the block-container padding + `TextEdit`'s internal
    // `CONTENT_PAD`, so the dots land at the same x as message body text.
    if status.is_streaming {
        chat_col = chat_col.push(
            container(streaming_indicator::view(status.esc_count))
                .padding([theme::SPACING_SM, theme::SPACING_MD + theme::SPACING_SM])
                .width(Length::Fill),
        );
    }

    // Obvious chrome after transcript content, inside the scroll column.
    // Optional top pad pins chips to the bottom of the viewport when history
    // is short; when history already fills the viewport, pad is 0 and chips
    // sit naturally after the last message. Keeping chrome inside the scroll
    // (not between scroll and composer) preserves a stable outer widget tree
    // so the input keeps focus when chrome shows/hides.
    let input_empty = input_value.text().trim().is_empty();
    if crate::obvious_bubble::chrome_visible(
        status.is_streaming,
        input_empty,
        obvious_chrome,
    ) {
        if chrome_top_pad > 0.0 {
            chat_col = chat_col.push(
                Space::new()
                    .width(Length::Fill)
                    .height(chrome_top_pad),
            );
        }
        chat_col = chat_col.push(view_obvious_chrome(obvious_chrome));
    }

    let chat_scroll = scrollable(chat_col)
        .direction(theme::thin_scrollbar_direction())
        .style(theme::thin_scrollbar)
        .width(Length::Fill)
        .height(Length::Fill)
        .on_scroll(Msg::ChatScrolled)
        .id(CHAT_SCROLLABLE_ID);
    let chat_area = container(chat_scroll)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::chat_area);

    // Completion popup — always rendered with the same widget type so iced's
    // tree diff preserves input focus. When hidden, the inner column is
    // empty and the background is suppressed so the popup collapses cleanly.
    // When shown it shares the chat input's "paper" bg so the popup reads as
    // a continuation of the input field with a top hairline separating it
    // from the chat transcript.
    let has_completion = completion.visible && {
        let input_text = input_value.text();
        let query = input_text.trim_start_matches('/');
        !filter_commands(commands, query).is_empty()
    };
    let completion_col = if has_completion {
        let input_text = input_value.text();
        let query = input_text.trim_start_matches('/');
        let filtered = filter_commands(commands, query);
        let mut col = column![].spacing(0.0);
        col = col.push(completion_divider());
        col = col.push(view_completion_col(
            commands,
            &filtered,
            completion.selected,
        ));
        col
    } else {
        column![].spacing(0.0)
    };
    let completion_el: Element<'a, Msg> = container(completion_col)
        .width(Length::Fill)
        .style(move |_theme: &iced::Theme| {
            if has_completion {
                container::Style {
                    background: Some(iced::Background::Color(theme::bg_base())),
                    ..Default::default()
                }
            } else {
                container::Style::default()
            }
        })
        .into();

    // Input area — promoted to the custom TextEdit widget so prompts get
    // markdown syntax highlighting and the full editor toolkit (undo,
    // word-nav, selection). Plain Enter sends via `on_submit`; empty Cmd+Enter
    // sends oneshot; Shift+Enter inserts a newline. Grows via `fit_content`.
    // Display-only key prefixes on ghost / oneshot text; send paths never
    // include the markers (next_actions.send / oneshot_cmd_submit_text).
    let show_tab_marker = crate::default_prompts::next_tab_marker_visible(
        input_empty,
        status.is_streaming,
        next_actions.len(),
    );
    let ghost_body = crate::default_prompts::next_ghost_text(
        status.is_streaming,
        next_actions,
        next_action_idx,
    )
    .unwrap_or("");
    let ghost = if ghost_body.is_empty() {
        String::new()
    } else if show_tab_marker {
        format!("⇥  {ghost_body}")
    } else {
        ghost_body.to_string()
    };

    let mut input = text_edit::TextEdit::new(input_value, Msg::InputAction)
        .id(CHAT_INPUT_ID)
        .show_gutter(false)
        .word_wrap(true)
        .fit_content(true)
        .max_rows(CHAT_INPUT_MAX_ROWS)
        .transparent_bg(true)
        .on_submit(Msg::SendPressed)
        .on_empty_cmd_submit(Msg::SendOneshotSuggestion);
    if !ghost.is_empty() {
        input = input.placeholder(ghost);
    }

    // Oneshot chrome under the input only when empty and not mid-turn:
    // loading while pending, list when ready. Never lists next-card actions.
    let defaults_chrome = crate::default_prompts::defaults_chrome(
        input_empty,
        default_prompts_pending,
        status.is_streaming,
        oneshot_prompts.len(),
    );
    let defaults_chrome_el: Option<Element<'a, Msg>> = match defaults_chrome {
        crate::default_prompts::DefaultsChrome::Hidden => None,
        crate::default_prompts::DefaultsChrome::Loading => {
            Some(view_default_prompts_loading())
        }
        crate::default_prompts::DefaultsChrome::List => oneshot_prompts
            .first()
            .map(|s| view_oneshot_suggestion(s)),
    };

    let input_divider = rule::horizontal(1).style(|_theme: &iced::Theme| rule::Style {
        color: theme::border_color(),
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
        snap: true,
    });

    // Meta row — model + context tokens — sits inside the input container
    // below the editor, blending into the "paper" surface (à la Zed). The
    // extra `SPACING_SM` horizontal padding lines the meta text up with the
    // input's own text (container XS + TextEdit CONTENT_PAD = 12px).
    // Fill is measured against the *selected* model's window (`context_max`).
    // An unknown window yields no fill — raw token count only, no percentage.
    let ctx_pct = context_fill(status.context_tokens, status.context_max)
        .map(|fill| (fill * 100.0) as usize);
    let ctx_color = match ctx_pct {
        Some(pct) if pct >= 90 => theme::error(),
        Some(pct) if pct >= 75 => theme::warning(),
        _ => theme::text_muted(),
    };
    let has_attachments = !pinned_selections.is_empty() || tentative_selection.is_some();
    let mut meta_inner = row![]
        .spacing(theme::SPACING_SM)
        .align_y(iced::Alignment::Center);
    if has_attachments {
        meta_inner = meta_inner.push(
            text("⌘R reset")
                .size(theme::font_sm())
                .color(theme::text_muted()),
        );
    }
    meta_inner = meta_inner.push(Space::new().width(Length::Fill));
    // Resend-history hint: only when the next send would actually re-feed the
    // transcript (no resumable session *and* non-empty messages).
    if show_resend_history_hint(status.will_resume, !session.messages.is_empty()) {
        meta_inner = meta_inner.push(
            text("⟳ resends full history")
                .size(theme::font_sm())
                .color(theme::accent()),
        );
    }
    // Closed control shows the short display name; menu options keep
    // harness-prefixed `label` via Display. Equality is on (harness, id).
    let mut selected_closed = status.selected_model.clone();
    selected_closed.label = selected_closed.closed_label.clone();
    meta_inner = meta_inner.push(
        pick_list(
            status.model_choices,
            Some(selected_closed),
            Msg::ModelSelected,
        )
        .text_size(theme::font_sm())
        .padding([0.0, theme::SPACING_XS])
        .style(theme::pick_list_ghost_style)
        .menu_style(theme::pick_list_menu),
    );
    let ctx_label = match status.context_max {
        // Progressive readout when the window is known and positive.
        Some(max) if max > 0 => format_usage_readout(status.context_tokens, max),
        // No known window → raw token count with no fill.
        _ => format_number(status.context_tokens),
    };
    meta_inner = meta_inner.push(text(ctx_label).size(theme::font_sm()).color(ctx_color));
    // Extra top padding separates the prompt from the meta strip so the
    // toolbar doesn't crowd the last input line.
    let meta_row = container(meta_inner)
        .padding(iced::Padding {
            top: theme::SPACING_SM,
            right: theme::SPACING_SM,
            bottom: 0.0,
            left: theme::SPACING_SM,
        })
        .width(Length::Fill);

    // Build the composer column without zero-height placeholders — empty
    // `Space` siblings still consume `column` spacing and inflated the gap
    // above the default-prompt strip.
    let mut composer_col = column![].spacing(theme::SPACING_XS);

    // Queue pill — renders above the input when a message is staged while the
    // agent is still streaming. Uses a read-only TextEdit so it matches the
    // shape of regular chat messages.
    if let Some(ed) = queue_editor {
        let editor = text_edit::TextEdit::new(ed, Msg::QueueAction)
            .show_gutter(false)
            .word_wrap(true)
            .read_only(true)
            .fit_content(true)
            .transparent_bg(true);
        let close_btn = button(
            text("×")
                .size(theme::content_size())
                .color(theme::text_muted()),
        )
        .on_press(Msg::DiscardQueue)
        .padding([0.0, theme::SPACING_XS])
        .style(|_theme, _status| iced::widget::button::Style {
            background: None,
            ..Default::default()
        });
        let label = text("Queued (enter to interrupt and send, backspace to cancel)")
            .size(theme::font_sm())
            .color(theme::text_muted());
        let header_row = row![
            container(label).width(Length::Fill),
            container(close_btn).align_y(iced::Alignment::Start),
        ]
        .spacing(theme::SPACING_XS)
        .align_y(iced::Alignment::Center);
        let pill_col = column![header_row, container(editor).width(Length::Fill)]
            .spacing(theme::SPACING_XS);
        composer_col = composer_col.push(
            container(pill_col)
                .padding([theme::SPACING_SM, theme::SPACING_MD])
                .width(Length::Fill)
                .style(theme::chat_queued_card),
        );
    }

    // Selection-context chips: pinned first, then the live tentative slot.
    // Iced 0.14's `Row::wrap()` lays children out across multiple lines
    // when they overflow horizontally, so a long chip set grows upward
    // above the input rather than forcing a horizontal scroll. Tab-source
    // labels are abbreviated to filename + minimal disambiguating
    // parents — long paths would otherwise get truncated by ellipsis.
    if has_attachments {
        let mut all: Vec<&SelectionContext> = pinned_selections.iter().collect();
        if let Some(t) = tentative_selection {
            all.push(t);
        }
        let labels = interaction::chip_labels_abbreviated(&all);
        let pinned_count = pinned_selections.len();
        let mut chips: Vec<Element<'a, Msg>> = Vec::with_capacity(all.len());
        for (i, label) in labels.into_iter().enumerate() {
            let tentative = i >= pinned_count;
            chips.push(view_selection_chip(label, tentative));
        }
        let wrapped = iced::widget::Row::with_children(chips)
            .spacing(theme::SPACING_XS)
            .align_y(iced::Alignment::Center)
            .wrap()
            .vertical_spacing(theme::SPACING_XS);
        composer_col = composer_col.push(
            container(wrapped)
                .padding([0.0, theme::SPACING_SM])
                .width(Length::Fill),
        );
    }

    composer_col = composer_col.push(input);
    if let Some(chrome) = defaults_chrome_el {
        composer_col = composer_col.push(chrome);
    }
    composer_col = composer_col.push(meta_row);

    // Horizontal padding here sums with TextEdit's internal CONTENT_PAD (8px)
    // to land the input's text at the same 12px the chat headers use.
    let input_row = container(composer_col)
        .padding([theme::SPACING_SM, theme::SPACING_XS])
        .width(Length::Fill)
        .style(theme::chat_input);

    // Stable outer column (scroll → completion → divider → input) so showing
    // or hiding in-scroll chrome never remounts the input and steals focus.
    column![chat_area, completion_el, input_divider, input_row]
        .height(Length::Fill)
        .into()
}

/// Measure the chat scrollable's viewport and content heights via a widget
/// operation. Unlike `on_scroll`, this runs even when content fits the
/// viewport (iced suppresses scroll notifications in that case).
pub fn measure_scroll_bounds() -> iced::Task<(f32, f32)> {
    use iced::Rectangle;
    use iced::Vector;
    use iced::advanced::widget::Id;
    use iced::advanced::widget::operation::{self, Operation, Outcome};

    struct Measure {
        viewport_h: Option<f32>,
        content_h: Option<f32>,
    }

    impl Operation<(f32, f32)> for Measure {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<(f32, f32)>)) {
            operate(self);
        }

        fn scrollable(
            &mut self,
            id: Option<&Id>,
            bounds: Rectangle,
            content_bounds: Rectangle,
            _translation: Vector,
            _state: &mut dyn operation::Scrollable,
        ) {
            if id == Some(&Id::new(CHAT_SCROLLABLE_ID)) {
                self.viewport_h = Some(bounds.height);
                self.content_h = Some(content_bounds.height);
            }
        }

        fn finish(&self) -> Outcome<(f32, f32)> {
            match (self.viewport_h, self.content_h) {
                (Some(v), Some(c)) if v > 0.0 => Outcome::Some((v, c)),
                _ => Outcome::None,
            }
        }
    }

    iced::advanced::widget::operate(Measure {
        viewport_h: None,
        content_h: None,
    })
}

/// Render a single chat block, Zed-style calm transcript:
///
/// - **User** (normal): bordered card on the "paper" surface (no label, no chevron).
/// - **User** (priming Setup): collapsible muted header; user-card body when open.
/// - **Answer / System**: plain text flowing on the chat background.
/// - **Thinking**: muted collapsible header; body when expanded.
/// - **Activity**: framed group card with quiet tool rows when expanded.
fn view_block<'a>(
    idx: usize,
    block: &'a Block,
    editor: Option<&'a EditorState>,
    collapsed: bool,
    hl_ranges: Vec<text_edit::HighlightRange>,
    hl_current: Option<text_edit::HighlightRange>,
) -> Element<'a, Msg> {
    match block.kind {
        BlockKind::Reasoning => {
            view_thinking_block(idx, block, editor, collapsed, hl_ranges, hl_current)
        }
        BlockKind::Activity | BlockKind::ToolUse | BlockKind::ToolResult => {
            view_activity_block(idx, block, editor, collapsed, hl_ranges, hl_current)
        }
        BlockKind::User if block.is_priming => {
            view_priming_user_block(idx, block, editor, collapsed, hl_ranges, hl_current)
        }
        BlockKind::User | BlockKind::Assistant | BlockKind::System => {
            view_prose_block(idx, block, editor, hl_ranges, hl_current)
        }
    }
}

/// User / Answer / System: no header, no chevron.
fn view_prose_block<'a>(
    idx: usize,
    block: &'a Block,
    editor: Option<&'a EditorState>,
    hl_ranges: Vec<text_edit::HighlightRange>,
    hl_current: Option<text_edit::HighlightRange>,
) -> Element<'a, Msg> {
    let has_content = !block.lines.is_empty();
    if !has_content {
        return Space::new().into();
    }
    let Some(ed) = editor else {
        return Space::new().into();
    };

    let content = text_edit::TextEdit::new(ed, move |action| Msg::ChatAction(idx, action))
        .show_gutter(false)
        .word_wrap(true)
        .md_tables(true)
        .read_only(true)
        .fit_content(true)
        .transparent_bg(true)
        .highlights(hl_ranges, hl_current);

    let padded = container(content)
        .padding([theme::SPACING_SM, theme::SPACING_MD])
        .width(Length::Fill);

    match block.kind {
        BlockKind::User => container(padded.style(theme::chat_user_card))
            .padding([0.0, theme::SPACING_SM])
            .width(Length::Fill)
            .into(),
        _ => padded.into(),
    }
}

/// Priming Setup user message: collapsible so scroll-to-top skips the
/// AGENTS.md / orientation inject. Starts collapsed; expand on click;
/// auto-hides again after [`PRIMING_RECOLLAPSE_SECS`].
fn view_priming_user_block<'a>(
    idx: usize,
    block: &'a Block,
    editor: Option<&'a EditorState>,
    collapsed: bool,
    hl_ranges: Vec<text_edit::HighlightRange>,
    hl_current: Option<text_edit::HighlightRange>,
) -> Element<'a, Msg> {
    let has_content = !block.lines.is_empty();
    let body_shown = has_content && !collapsed && editor.is_some();
    let header_label = if collapsed {
        priming_collapsed_label(&block.lines)
    } else {
        block.label.clone()
    };
    let label = text(header_label)
        .size(theme::content_size())
        .font(theme::content_font())
        .color(theme::text_muted());
    let header_row = row![collapsible::chevron(!collapsed), label]
        .spacing(theme::SPACING_XS)
        .align_y(iced::Alignment::Center);
    let header_content: Element<'a, Msg> = button(header_row)
        .on_press(Msg::ToggleCollapse(idx))
        .padding(0.0)
        .style(|_theme, _status| iced::widget::button::Style {
            background: None,
            ..Default::default()
        })
        .into();
    let header = container(header_content)
        .padding([theme::SPACING_XS, theme::SPACING_MD])
        .width(Length::Fill);

    let mut col = column![header].width(Length::Fill);
    if body_shown && let Some(ed) = editor {
        let content = text_edit::TextEdit::new(ed, move |action| Msg::ChatAction(idx, action))
            .show_gutter(false)
            .word_wrap(true)
            .md_tables(true)
            .read_only(true)
            .fit_content(true)
            .transparent_bg(true)
            .highlights(hl_ranges, hl_current);
        let padded = container(content)
            .padding([theme::SPACING_SM, theme::SPACING_MD])
            .width(Length::Fill)
            .style(theme::chat_user_card);
        col = col.push(
            container(padded)
                .padding([0.0, theme::SPACING_SM])
                .width(Length::Fill),
        );
    }

    container(col)
        .padding([0.0, theme::SPACING_SM])
        .width(Length::Fill)
        .into()
}

/// Thinking: collapsible muted header; expanded body is the thought text.
fn view_thinking_block<'a>(
    idx: usize,
    block: &'a Block,
    editor: Option<&'a EditorState>,
    collapsed: bool,
    hl_ranges: Vec<text_edit::HighlightRange>,
    hl_current: Option<text_edit::HighlightRange>,
) -> Element<'a, Msg> {
    let has_content = !block.lines.is_empty();
    let body_shown = has_content && !collapsed && editor.is_some();
    let header_label = if collapsed {
        thinking_collapsed_label(&block.lines)
    } else {
        block.label.clone()
    };
    let label_color = theme::text_muted();

    let label = text(header_label)
        .size(theme::content_size())
        .font(theme::content_font())
        .color(label_color);
    let header_row = row![collapsible::chevron(!collapsed), label]
        .spacing(theme::SPACING_XS)
        .align_y(iced::Alignment::Center);
    let header_content: Element<'a, Msg> = button(header_row)
        .on_press(Msg::ToggleCollapse(idx))
        .padding(0.0)
        .style(|_theme, _status| iced::widget::button::Style {
            background: None,
            ..Default::default()
        })
        .into();

    let header = container(header_content)
        .padding([theme::SPACING_XS, theme::SPACING_MD])
        .width(Length::Fill);

    let mut col = column![header].width(Length::Fill);
    if body_shown && let Some(ed) = editor {
        let body = container(
            text_edit::TextEdit::new(ed, move |action| Msg::ChatAction(idx, action))
                .show_gutter(false)
                .word_wrap(true)
                .md_tables(true)
                .read_only(true)
                .fit_content(true)
                .transparent_bg(true)
                .base_color(theme::text_secondary())
                .highlights(hl_ranges, hl_current),
        )
        .padding(iced::Padding {
            top: 0.0,
            right: theme::SPACING_MD,
            bottom: theme::SPACING_SM,
            left: theme::SPACING_MD,
        })
        .width(Length::Fill);
        col = col.push(body);
    }

    container(col)
        .padding([0.0, theme::SPACING_SM])
        .width(Length::Fill)
        .into()
}

/// Activity group: framed card with summary header and quiet tool rows.
fn view_activity_block<'a>(
    idx: usize,
    block: &'a Block,
    editor: Option<&'a EditorState>,
    collapsed: bool,
    hl_ranges: Vec<text_edit::HighlightRange>,
    hl_current: Option<text_edit::HighlightRange>,
) -> Element<'a, Msg> {
    let label_color = block_header_color(block.kind);
    let has_content = !block.lines.is_empty();
    let body_shown = has_content && !collapsed && editor.is_some();

    // Activity headers use the content (monospace) font so tool names and
    // paths read like code, matching the quiet-row body below.
    let header_content: Element<'a, Msg> = {
        let label = text(&block.label)
            .size(theme::content_size())
            .font(theme::content_font())
            .color(label_color);
        let header_row = row![collapsible::chevron(!collapsed), label]
            .spacing(theme::SPACING_XS)
            .align_y(iced::Alignment::Center);
        button(header_row)
            .on_press(Msg::ToggleCollapse(idx))
            .padding(0.0)
            .style(|_theme, _status| iced::widget::button::Style {
                background: None,
                ..Default::default()
            })
            .into()
    };

    let header_style = if body_shown {
        theme::chat_tool_card_header_open
    } else {
        theme::chat_tool_card_header_alone
    };
    let header = container(header_content)
        .padding([theme::SPACING_SM, theme::SPACING_MD])
        .width(Length::Fill)
        .style(header_style);

    let mut card_col = column![header].width(Length::Fill);

    if body_shown && let Some(ed) = editor {
        let body = container(
            text_edit::TextEdit::new(ed, move |action| Msg::ChatAction(idx, action))
                .show_gutter(false)
                .word_wrap(true)
                .md_tables(true)
                .read_only(true)
                .fit_content(true)
                .transparent_bg(true)
                .highlights(hl_ranges, hl_current),
        )
        .padding([theme::SPACING_SM, theme::SPACING_MD])
        .width(Length::Fill)
        .style(theme::chat_tool_card_body);
        card_col = card_col.push(body);
    }

    // Outer: stack the column underneath a transparent-bg, border-only
    // overlay so the 1px frame draws on top of the header/body surfaces.
    let border_overlay = container(Space::new())
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::chat_tool_card_frame);
    let framed = stack![card_col, border_overlay];

    container(framed)
        .padding([0.0, theme::SPACING_SM])
        .width(Length::Fill)
        .into()
}

/// One selection-context chip — a small bordered label sitting above the
/// chat input. `tentative` chips use a muted border to signal "not yet
/// pinned (Cmd-K to keep)"; pinned chips use the primary border color.
fn view_selection_chip<'a>(label: String, tentative: bool) -> Element<'a, Msg> {
    let style = if tentative {
        theme::selection_chip_tentative
    } else {
        theme::selection_chip_pinned
    };
    let color = if tentative {
        theme::text_secondary()
    } else {
        theme::text_primary()
    };
    container(
        text(label)
            .size(theme::font_sm())
            .color(color)
            .wrapping(iced::widget::text::Wrapping::None),
    )
    .padding([2.0, theme::SPACING_SM])
    .style(style)
    .into()
}

/// Header label color for a block kind (re-exported from text_edit for convenience).
fn block_header_color(kind: BlockKind) -> iced::Color {
    match kind {
        BlockKind::User => theme::accent(),
        BlockKind::Assistant => theme::text_secondary(),
        BlockKind::Reasoning => theme::text_muted(),
        // Activity sits in a neutral palette so tool names stay legible
        // without competing with the accent-colored User card.
        BlockKind::Activity | BlockKind::ToolUse => theme::text_primary(),
        BlockKind::ToolResult => theme::text_secondary(),
        BlockKind::System => theme::text_muted(),
    }
}

fn format_number(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            result.push(',');
        }
        result.push(ch);
    }
    result
}

/// Tint for an obvious-chrome chip background.
#[derive(Clone, Copy)]
enum ObviousChipTone {
    /// Numbered options (⌘1…⌘n) — quiet light blue.
    Numbered,
    /// Cancel (⌘⌫).
    Reject,
}

/// Option chrome: numbered chips then optional cancel. View chrome only until
/// activation sends.
fn view_obvious_chrome<'a>(
    chrome: &'a crate::obvious_bubble::ObviousChrome,
) -> Element<'a, Msg> {
    use crate::obvious_bubble::{cancel_chip_label, option_chip_label};

    let mut col = column![].spacing(theme::SPACING_XS);

    for (i, action) in chrome.options.iter().enumerate() {
        col = col.push(view_obvious_chip(
            option_chip_label(i + 1, action),
            action.clone(),
            ObviousChipTone::Numbered,
        ));
    }

    if let Some(cancel) = chrome.cancel.as_deref() {
        col = col.push(view_obvious_chip(
            cancel_chip_label(cancel),
            cancel.to_string(),
            ObviousChipTone::Reject,
        ));
    }

    container(col)
        .padding([0.0, theme::SPACING_SM])
        .width(Length::Fill)
        .into()
}

/// One action chip: hotkey-first label; click sends `action` only.
/// Label ink uses secondary text (full alpha) for readable contrast on quiet
/// tinted fills in both light and dark themes — shared for all tones.
fn view_obvious_chip<'a>(
    label: String,
    action: String,
    tone: ObviousChipTone,
) -> Element<'a, Msg> {
    let body = text(label)
        .size(theme::content_size())
        .color(theme::text_secondary())
        .font(theme::content_font());

    let card = container(body)
        .padding([theme::SPACING_SM, theme::SPACING_MD])
        .width(Length::Fill)
        .style(move |t| match tone {
            ObviousChipTone::Numbered => theme::chat_obvious_chip_numbered(t),
            ObviousChipTone::Reject => theme::chat_obvious_chip_reject(t),
        });

    button(card)
        .on_press(Msg::SendObviousAction(action))
        .padding(0.0)
        .width(Length::Fill)
        .style(|_theme, status| {
            let base = iced::widget::button::Style {
                background: None,
                ..Default::default()
            };
            match status {
                iced::widget::button::Status::Hovered
                | iced::widget::button::Status::Pressed => base,
                _ => base,
            }
        })
        .into()
}

/// Quiet loading strip while the reply-suggestion oneshot is pending.
fn view_default_prompts_loading<'a>() -> Element<'a, Msg> {
    const CONTENT_PAD: f32 = 8.0;
    container(
        text("…")
            .size(theme::content_size())
            .color(theme::text_muted())
            .font(theme::content_font()),
    )
    .padding(iced::Padding {
        top: text_edit::CONTENT_PAD_Y,
        right: 0.0,
        bottom: text_edit::CONTENT_PAD_Y,
        left: CONTENT_PAD,
    })
    .width(Length::Fill)
    .into()
}

/// Under-input oneshot: display-only `⌘↩` prefix before the suggestion text.
/// Soft-wraps; full text stays visible. Send uses the bare suggestion string.
fn view_oneshot_suggestion<'a>(suggestion: &str) -> Element<'a, Msg> {
    const CONTENT_PAD: f32 = 8.0;
    let color = theme::text_muted();
    let label = format!(
        "{}  {suggestion}",
        crate::default_prompts::ONESHOT_CMD_ENTER_MARKER
    );
    container(
        text(label)
            .size(theme::content_size())
            .color(color)
            // UI font so ⌘ renders; content monospace often lacks the glyph.
            .font(theme::ui_font())
            .width(Length::Fill)
            .wrapping(Wrapping::Word),
    )
    .padding(iced::Padding {
        top: text_edit::CONTENT_PAD_Y,
        right: 0.0,
        bottom: text_edit::CONTENT_PAD_Y,
        left: CONTENT_PAD,
    })
    .width(Length::Fill)
    .into()
}


// view_status_bar removed: model + context now blend into the input area
// (see `view`), and stream state is conveyed by the streaming indicator.

// ── Completion popup ────────────────────────────────────────────────────────

fn view_completion_col<'a>(
    commands: &'a [SlashCommand],
    filtered: &[(usize, i32)],
    selected: usize,
) -> iced::widget::Column<'a, Msg> {
    let mut items = column![].spacing(0.0);
    for (i, &(cmd_idx, _score)) in filtered.iter().enumerate() {
        let cmd = &commands[cmd_idx];
        let is_selected = i == selected;
        let label = row![
            text(format!("/{}", cmd.name))
                .size(theme::font_sm())
                .color(theme::text_primary()),
            Space::new().width(theme::SPACING_SM),
            text(&cmd.description)
                .size(theme::font_sm())
                .color(theme::text_muted()),
        ]
        .align_y(iced::Alignment::Center);
        items = items.push(
            container(label)
                .width(Length::Fill)
                .padding([theme::SPACING_XS, theme::SPACING_MD])
                .style(move |_theme: &iced::Theme| {
                    if is_selected {
                        container::Style {
                            background: Some(iced::Background::Color(theme::bg_list_hover())),
                            ..Default::default()
                        }
                    } else {
                        container::Style::default()
                    }
                }),
        );
    }
    items
}

/// Hairline separator used at the top of the completion popup so it reads
/// as a distinct surface sitting above the chat transcript.
fn completion_divider<'a>() -> Element<'a, Msg> {
    rule::horizontal(1)
        .style(|_theme: &iced::Theme| rule::Style {
            color: theme::border_color(),
            radius: 0.0.into(),
            fill_mode: rule::FillMode::Full,
            snap: true,
        })
        .into()
}

// ── Fuzzy matching ──────────────────────────────────────────────────────────

/// Filter and score commands by fuzzy-matching `query` against command names.
/// Returns `(index_into_commands, score)` sorted by descending score.
pub fn filter_commands(commands: &[SlashCommand], query: &str) -> Vec<(usize, i32)> {
    let mut matches: Vec<(usize, i32)> = commands
        .iter()
        .enumerate()
        .filter_map(|(i, cmd)| fuzzy_score(query, &cmd.name).map(|s| (i, s)))
        .collect();
    matches.sort_by(|a, b| b.1.cmp(&a.1));
    matches
}

/// Subsequence fuzzy match. Returns `None` if `query` is not a subsequence of
/// `target`, otherwise a score (higher = better).
fn fuzzy_score(query: &str, target: &str) -> Option<i32> {
    if query.is_empty() {
        return Some(0);
    }
    let query_lower: Vec<char> = query.to_lowercase().chars().collect();
    let target_lower: Vec<char> = target.to_lowercase().chars().collect();
    let mut qi = 0;
    let mut score = 0i32;
    let mut prev_match = false;

    for (i, &ch) in target_lower.iter().enumerate() {
        if qi < query_lower.len() && ch == query_lower[qi] {
            qi += 1;
            score += 1;
            if i == 0 {
                score += 3; // bonus for matching start
            }
            if prev_match {
                score += 2; // bonus for consecutive
            }
            prev_match = true;
        } else {
            prev_match = false;
        }
    }

    if qi == query_lower.len() {
        Some(score)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(harness: &str, id: &str, window: Option<usize>) -> ModelInfo {
        ModelInfo {
            harness: harness.to_string(),
            id: id.to_string(),
            display: id.to_string(),
            context_window: window,
        }
    }

    /// @spec harness/model-picker Harness-grouped choices: Choices present each model under its harness
    #[test]
    fn choices_present_each_model_under_its_harness() {
        // GIVEN selectable models drawn from more than one harness.
        let models = vec![
            model("claude-code", "opus", None),
            model("grok", "grok-4.5", Some(256_000)),
            model("claude-code", "sonnet", None),
        ];
        // WHEN the picker choices are built.
        let choices = group_choices(models);
        // THEN each model appears under its owning harness — the choice carries
        // its model's harness and its label is presented under that harness.
        let opus = choices.iter().find(|c| c.id.as_deref() == Some("opus")).unwrap();
        assert_eq!(opus.harness.as_deref(), Some("claude-code"));
        assert!(opus.label.starts_with("Claude Code · "));
        let grok = choices
            .iter()
            .find(|c| c.id.as_deref() == Some("grok-4.5"))
            .unwrap();
        assert_eq!(grok.harness.as_deref(), Some("grok"));
        assert!(grok.label.starts_with("Grok · "));
        // AND a harness's models stay contiguous rather than interleaved.
        let harnesses: Vec<&str> = choices
            .iter()
            .map(|c| c.harness.as_deref().unwrap())
            .collect();
        assert_eq!(harnesses, ["claude-code", "claude-code", "grok"]);
    }

    /// @spec harness/model-picker Context fill from the active model's window: Fill is measured against the selected model's window
    #[test]
    fn fill_is_measured_against_the_selected_models_window() {
        // GIVEN a selected model with a known context window AND a used-token count.
        let window = Some(200_000);
        let used = 50_000;
        // WHEN the usage meter fill is computed.
        let fill = context_fill(used, window);
        // THEN the fill is the used tokens relative to that model's window.
        assert_eq!(fill, Some(0.25));
    }

    /// @spec harness/model-picker Context fill from the active model's window: A model with no known window shows no fill
    #[test]
    fn a_model_with_no_known_window_shows_no_fill() {
        // GIVEN a selected model with no known context window.
        // WHEN the usage meter fill is computed.
        let fill = context_fill(50_000, None);
        // THEN the meter shows no fill.
        assert_eq!(fill, None);
    }

    /// @spec chat/composer-footer Resend hint only when history would be resent: Hint shown when history would be resent
    #[test]
    fn hint_shown_when_history_would_be_resent() {
        // GIVEN a chat with no resumable agent session AND a non-empty transcript.
        let will_resume = false;
        let has_messages = true;
        // WHEN the composer footer is rendered (hint visibility is computed).
        let show = show_resend_history_hint(will_resume, has_messages);
        // THEN the resend-history hint is shown.
        assert!(show);
    }

    /// @spec chat/composer-footer Resend hint only when history would be resent: Hint hidden when next send would resume
    #[test]
    fn hint_hidden_when_next_send_would_resume() {
        // GIVEN a chat with a resumable agent session AND a non-empty transcript.
        let will_resume = true;
        let has_messages = true;
        // WHEN the composer footer is rendered.
        let show = show_resend_history_hint(will_resume, has_messages);
        // THEN the resend-history hint is not shown.
        assert!(!show);
    }

    /// @spec chat/composer-footer Resend hint only when history would be resent: Hint hidden when transcript is empty
    #[test]
    fn hint_hidden_when_transcript_is_empty() {
        // GIVEN a chat with no resumable agent session AND an empty transcript.
        let will_resume = false;
        let has_messages = false;
        // WHEN the composer footer is rendered.
        let show = show_resend_history_hint(will_resume, has_messages);
        // THEN the resend-history hint is not shown.
        assert!(!show);
    }

    /// @spec chat/composer-footer Progressive usage readout: Cool fill shows percentage only
    #[test]
    fn cool_fill_shows_percentage_only() {
        // GIVEN a known context window AND used tokens such that fill is below 75%.
        let window = 200_000;
        let used = 50_000; // 25%
        // WHEN the usage readout is formatted.
        let readout = format_usage_readout(used, window);
        // THEN the readout shows the fill percentage AND does not include absolute used or max.
        assert_eq!(readout, "25%");
        assert!(!readout.contains('/'));
        assert!(!readout.contains(','));
    }

    /// @spec chat/composer-footer Progressive usage readout: Hot fill shows used, max, and percentage
    #[test]
    fn hot_fill_shows_used_max_and_percentage() {
        // GIVEN a known context window AND used tokens such that fill is at least 75%.
        let window = 200_000;
        let used = 150_000; // 75%
        // WHEN the usage readout is formatted.
        let readout = format_usage_readout(used, window);
        // THEN the readout includes used tokens, the window max, and the fill percentage.
        assert_eq!(readout, "150,000 / 200,000 (75%)");
    }

    /// @spec chat/composer-footer Short closed model label: Closed label is the model display name
    #[test]
    fn closed_label_is_the_model_display_name() {
        // GIVEN a selectable model with a harness name and a short display name.
        let models = vec![ModelInfo {
            harness: "grok".to_string(),
            id: "grok-4.5".to_string(),
            display: "Grok 4.5".to_string(),
            context_window: Some(500_000),
        }];
        // WHEN the closed model control label is built (with menu choices).
        let choices = group_choices(models);
        let choice = choices.first().expect("one choice");
        // THEN the closed label is the short display name AND does not include a harness prefix.
        assert_eq!(choice.closed_label, "Grok 4.5");
        assert!(!choice.closed_label.contains('·'));
        // Menu label remains harness-prefixed for grouped choices.
        assert!(choice.label.starts_with("Grok · "));
    }

    #[test]
    fn strips_csi_color_codes() {
        let input = "\x1B[32mcreated \x1B[39m changes/foo/proposal.md";
        assert_eq!(
            strip_ansi_escapes(input),
            "created  changes/foo/proposal.md"
        );
    }

    #[test]
    fn strips_tool_use_error_tags() {
        let input = "<tool_use_error>File has not been read yet.</tool_use_error>";
        assert_eq!(
            strip_tool_wrapper_tags(input),
            "File has not been read yet.",
        );
    }

    #[test]
    fn truncate_output_cleans_color_and_tags() {
        let raw = "\x1B[32mcreated \x1B[39m changes/foo/proposal.md";
        assert_eq!(
            truncate_output(raw),
            vec!["created  changes/foo/proposal.md".to_string()],
        );

        let raw = "<tool_use_error>File has not been read yet.</tool_use_error>";
        assert_eq!(
            truncate_output(raw),
            vec!["File has not been read yet.".to_string()],
        );
    }

    #[test]
    fn truncate_chars_keeps_short_strings() {
        assert_eq!(truncate_chars("short", 40), "short");
        assert_eq!(truncate_chars("", 40), "");
    }

    #[test]
    fn truncate_chars_never_splits_multibyte() {
        // A run of multibyte chars whose byte length exceeds the limit: a byte
        // slice at `max` would land mid-character and panic. We must cut on a
        // char boundary and keep exactly `max` chars.
        let s = "日".repeat(60); // 60 three-byte chars
        let out = truncate_chars(&s, 40);
        assert_eq!(out.chars().count(), 40);
        assert!(s.starts_with(out));
    }

    #[test]
    fn tool_summary_handles_multibyte_pattern_and_command() {
        // Regression: these previously byte-sliced at 40/50 and aborted the
        // app when a multibyte char straddled the boundary.
        let long_pat = "—".repeat(60); // em-dash is 3 bytes each
        let input = format!(r#"{{"pattern":"{long_pat}"}}"#);
        let summary = format_tool_summary("Grep", &input);
        assert!(
            summary.starts_with("Grep · \"—"),
            "expected calm Grep label, got {summary}"
        );

        let long_cmd = "é".repeat(60); // 2 bytes each
        let input = format!(r#"{{"command":"{long_cmd}"}}"#);
        let summary = format_tool_summary("Bash", &input);
        assert!(
            summary.starts_with("Shell · é"),
            "Bash should map to Shell with command detail: {summary}"
        );
    }

    #[test]
    fn known_claude_and_grok_tools_share_calm_labels() {
        // Claude-style names
        assert_eq!(
            format_tool_summary("Read", r#"{"path":"crates/duckboard/src/widget/agent_chat.rs"}"#),
            "Read · src/widget/agent_chat.rs"
        );
        assert_eq!(
            format_tool_summary("Bash", r#"{"command":"cargo test -p duckboard"}"#),
            "Shell · cargo test -p duckboard"
        );
        assert_eq!(
            format_tool_summary("Grep", r#"{"pattern":"format_tool_summary"}"#),
            "Grep · \"format_tool_summary\""
        );
        assert_eq!(
            format_tool_summary("Edit", r#"{"file_path":"src/state.rs","old_string":"a","new_string":"b"}"#),
            "Edit · src/state.rs"
        );

        // Grok-style names — same verbs, same calm shape
        assert_eq!(
            format_tool_summary("read_file", r#"{"path":"foo.rs"}"#),
            "Read · foo.rs"
        );
        assert_eq!(
            format_tool_summary(
                "run_terminal_command",
                r#"{"command":"ds status"}"#
            ),
            "Shell · ds status"
        );
        assert_eq!(
            format_tool_summary(
                "search_replace",
                r#"{"path":"crates/duckboard/src/main.rs","old_string":"x","new_string":"y"}"#
            ),
            "Edit · duckboard/src/main.rs"
        );
    }

    #[test]
    fn unknown_tools_look_intentional_not_raw_json() {
        // Humanized name + one short detail; no JSON blob.
        let summary = format_tool_summary(
            "some_obscure_tool",
            r#"{"target":"widget","payload":{"nested":true},"contents":"a huge body\nwith lines"}"#,
        );
        assert_eq!(summary, "Some obscure tool · widget");
        assert!(!summary.contains('{'), "must not dump JSON: {summary}");
        assert!(!summary.contains("nested"), "must not dump nested objects: {summary}");

        // Empty / minimal input: name alone, still clean.
        assert_eq!(format_tool_summary("camelCaseThing", ""), "Camel case thing");
        assert_eq!(format_tool_summary("  ", r#"{}"#), "Tool");
        assert_eq!(
            format_tool_summary("run_mystery", r#"{"old_string":"a\nb","new_string":"c\nd"}"#),
            "Run mystery"
        );
    }

    #[test]
    fn collapsed_activity_uses_humanized_verbs() {
        let session = assistant_blocks(vec![
            tool_use("1", "read_file", r#"{"path":"a.rs"}"#),
            tool_result("1", "read_file", "ok"),
            tool_use("2", "run_terminal_command", r#"{"command":"ls"}"#),
            tool_result("2", "run_terminal_command", "file"),
            tool_use("3", "grep", r#"{"pattern":"x"}"#),
            tool_result("3", "grep", "hit"),
        ]);
        let segs = build_transcript_segments(&session);
        let tools = activity_tools(&segs);
        let label = activity_collapsed_label(tools);
        assert!(
            label.contains("Read") && label.contains("Shell") && label.contains("Grep"),
            "collapsed samples should use human verbs, not harness ids: {label}"
        );
        assert!(
            !label.contains("run_terminal") && !label.contains("read_file"),
            "raw harness ids must not appear: {label}"
        );
    }

    // ── Transcript segment builder ──────────────────────────────────────

    fn assistant_blocks(blocks: Vec<ContentBlock>) -> ChatSession {
        let mut s = ChatSession::new("test".into());
        s.messages.push(crate::chat_store::ChatMessage {
            role: Role::Assistant,
            content: blocks,
            timestamp: String::new(),
            is_priming: false,
        });
        s
    }

    fn tool_use(id: &str, name: &str, input: &str) -> ContentBlock {
        ContentBlock::ToolUse {
            id: id.into(),
            name: name.into(),
            input: input.into(),
        }
    }

    fn tool_result(id: &str, name: &str, output: &str) -> ContentBlock {
        ContentBlock::ToolResult {
            id: id.into(),
            name: name.into(),
            output: output.into(),
        }
    }

    fn activity_tools(segs: &[TranscriptSeg]) -> &[ToolRow] {
        segs.iter()
            .find_map(|s| match s {
                TranscriptSeg::Activity { tools, .. } => Some(tools.as_slice()),
                _ => None,
            })
            .expect("expected an Activity segment")
    }

    /// @spec chat/transcript Segment construction: Reasoning then answer yields Thinking then Answer
    #[test]
    fn reasoning_then_answer_yields_thinking_then_answer() {
        // GIVEN a session whose assistant content is a reasoning block
        // followed by a text block.
        let session = assistant_blocks(vec![
            ContentBlock::Reasoning("ponder the options".into()),
            ContentBlock::Text("here is the answer".into()),
        ]);

        // WHEN the transcript segments are built.
        let segs = build_transcript_segments(&session);

        // THEN the segments are a Thinking segment then an Answer segment
        // AND the reasoning body is not part of the Answer segment.
        assert_eq!(segs.len(), 2);
        match &segs[0] {
            TranscriptSeg::Thinking { lines, live } => {
                assert_eq!(lines, &["ponder the options".to_string()]);
                assert!(!*live);
            }
            other => panic!("expected Thinking, got {other:?}"),
        }
        match &segs[1] {
            TranscriptSeg::Answer { lines, live } => {
                assert_eq!(lines, &["here is the answer".to_string()]);
                assert!(!*live);
                assert!(!lines.iter().any(|l| l.contains("ponder")));
            }
            other => panic!("expected Answer, got {other:?}"),
        }
    }

    /// @spec chat/transcript Segment construction: Contiguous tools yield one Activity with multiple rows
    #[test]
    fn contiguous_tools_yield_one_activity_with_multiple_rows() {
        // GIVEN a session whose assistant content is several consecutive
        // tool uses and their results.
        let session = assistant_blocks(vec![
            tool_use("1", "Read", r#"{"path":"a.rs"}"#),
            tool_result("1", "Read", "fn a() {}"),
            tool_use("2", "grep", r#"{"pattern":"foo"}"#),
            tool_result("2", "grep", "match"),
            tool_use("3", "shell", r#"{"command":"ls"}"#),
            tool_result("3", "shell", "file"),
        ]);

        // WHEN the transcript segments are built.
        let segs = build_transcript_segments(&session);

        // THEN those tools form a single Activity segment AND the segment
        // has one row per tool call.
        assert_eq!(segs.len(), 1);
        let tools = activity_tools(&segs);
        assert_eq!(tools.len(), 3);
        assert_eq!(tools[0].id, "1");
        assert_eq!(tools[1].id, "2");
        assert_eq!(tools[2].id, "3");
    }

    /// @spec chat/transcript Segment construction: Thought, tools, thought, answer yields four segments in order
    #[test]
    fn thought_tools_thought_answer_yields_four_segments() {
        // GIVEN a session whose assistant content is reasoning, then tools,
        // then reasoning, then text.
        let session = assistant_blocks(vec![
            ContentBlock::Reasoning("first thought".into()),
            tool_use("t1", "Read", r#"{"path":"x"}"#),
            tool_result("t1", "Read", "ok"),
            ContentBlock::Reasoning("second thought".into()),
            ContentBlock::Text("final answer".into()),
        ]);

        // WHEN the transcript segments are built.
        let segs = build_transcript_segments(&session);

        // THEN the segments are Thinking, Activity, Thinking, Answer in order.
        assert_eq!(segs.len(), 4);
        assert!(matches!(&segs[0], TranscriptSeg::Thinking { lines, .. } if lines == &["first thought".to_string()]));
        assert!(matches!(&segs[1], TranscriptSeg::Activity { tools, .. } if tools.len() == 1));
        assert!(matches!(&segs[2], TranscriptSeg::Thinking { lines, .. } if lines == &["second thought".to_string()]));
        assert!(matches!(&segs[3], TranscriptSeg::Answer { lines, .. } if lines == &["final answer".to_string()]));
    }

    /// @spec chat/transcript Segment construction: Live pending reasoning appears on an open Thinking segment
    #[test]
    fn live_pending_reasoning_appears_on_open_thinking() {
        // GIVEN a streaming session with non-empty pending reasoning and no
        // committed reasoning for that run yet.
        let mut session = ChatSession::new("test".into());
        session.is_streaming = true;
        session.pending_reasoning = "still thinking…".into();

        // WHEN the transcript segments are built.
        let segs = build_transcript_segments(&session);

        // THEN a live Thinking segment includes that pending reasoning text.
        assert_eq!(segs.len(), 1);
        match &segs[0] {
            TranscriptSeg::Thinking { lines, live } => {
                assert!(*live);
                assert_eq!(lines, &["still thinking…".to_string()]);
            }
            other => panic!("expected live Thinking, got {other:?}"),
        }
    }

    /// @spec chat/transcript Segment construction: Live reasoning with an open answer draft yields Thinking then one Answer
    #[test]
    fn live_reasoning_with_open_answer_draft_yields_thinking_then_one_answer() {
        // GIVEN a streaming session with both pending reasoning and pending answer.
        let mut session = ChatSession::new("test".into());
        session.is_streaming = true;
        session.pending_reasoning = "rethinking the outline".into();
        session.pending_text = "first draft of the write-gate".into();

        // WHEN the transcript segments are built.
        let segs = build_transcript_segments(&session);

        // THEN Thinking then one Answer for that open draft (not stacked answers).
        // Thinking may auto-collapse (`live=false`) when Answer follows — that is
        // collapse policy, not multi-answer thrash.
        assert_eq!(segs.len(), 2);
        match &segs[0] {
            TranscriptSeg::Thinking { lines, .. } => {
                assert_eq!(lines, &["rethinking the outline".to_string()]);
            }
            other => panic!("expected Thinking first, got {other:?}"),
        }
        match &segs[1] {
            TranscriptSeg::Answer { lines, live } => {
                assert!(*live, "open draft Answer should be live while streaming");
                assert_eq!(lines, &["first draft of the write-gate".to_string()]);
            }
            other => panic!("expected Answer second, got {other:?}"),
        }
        let answer_count = segs
            .iter()
            .filter(|s| matches!(s, TranscriptSeg::Answer { .. }))
            .count();
        assert_eq!(answer_count, 1, "exactly one Answer for the open draft");
    }

    /// @spec chat/transcript Activity pairing: Matching use and result become one done row
    #[test]
    fn matching_use_and_result_become_one_done_row() {
        // GIVEN a tool use and a tool result that share the same call id.
        let session = assistant_blocks(vec![
            tool_use("call-1", "Read", r#"{"path":"src/main.rs"}"#),
            tool_result("call-1", "Read", "fn main() {}"),
        ]);

        // WHEN the transcript segments are built.
        let segs = build_transcript_segments(&session);

        // THEN the Activity segment has one done row for that id AND the
        // row carries the tool summary and the result body.
        let tools = activity_tools(&segs);
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].id, "call-1");
        assert_eq!(tools[0].status, ToolRowStatus::Done);
        assert!(tools[0].summary.contains("Read"));
        assert!(tools[0].summary.contains("main.rs"));
        assert_eq!(tools[0].output_lines, vec!["fn main() {}".to_string()]);
    }

    /// @spec chat/transcript Activity pairing: Non-adjacent use and result still pair by id
    #[test]
    fn non_adjacent_use_and_result_still_pair_by_id() {
        // GIVEN two tool uses and two results ordered so each result is not
        // immediately after its matching use, AND each result shares a call
        // id with exactly one of the uses.
        let session = assistant_blocks(vec![
            tool_use("a", "Read", r#"{"path":"a.rs"}"#),
            tool_use("b", "grep", r#"{"pattern":"x"}"#),
            tool_result("a", "Read", "contents of a"),
            tool_result("b", "grep", "match in b"),
        ]);

        // WHEN the transcript segments are built.
        let segs = build_transcript_segments(&session);

        // THEN each use is paired with its matching result into one done row
        // AND no row is labeled only as a generic done placeholder.
        let tools = activity_tools(&segs);
        assert_eq!(tools.len(), 2);
        let row_a = tools.iter().find(|t| t.id == "a").unwrap();
        let row_b = tools.iter().find(|t| t.id == "b").unwrap();
        assert_eq!(row_a.status, ToolRowStatus::Done);
        assert_eq!(row_b.status, ToolRowStatus::Done);
        assert_eq!(row_a.output_lines, vec!["contents of a".to_string()]);
        assert_eq!(row_b.output_lines, vec!["match in b".to_string()]);
        for row in tools {
            assert_ne!(row.summary, "✓ done");
            assert!(!row.summary.eq_ignore_ascii_case("done"));
        }
    }

    /// @spec chat/transcript Activity pairing: Orphan result is a named done row
    #[test]
    fn orphan_result_is_a_named_done_row() {
        // GIVEN a tool result with no preceding tool use for the same call id.
        let session = assistant_blocks(vec![tool_result("orphan-1", "Read", "file body")]);

        // WHEN the transcript segments are built.
        let segs = build_transcript_segments(&session);

        // THEN the Activity segment includes a done row labeled from the
        // result's tool name AND the row is not labeled only as a generic
        // done placeholder.
        let tools = activity_tools(&segs);
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].status, ToolRowStatus::Done);
        assert_eq!(tools[0].summary, "Read");
        assert_ne!(tools[0].summary, "✓ done");
        assert_eq!(tools[0].output_lines, vec!["file body".to_string()]);
    }

    // ── Segment presentation ────────────────────────────────────────────

    /// @spec chat/transcript Segment presentation: Thinking collapsed label includes line count
    #[test]
    fn thinking_collapsed_label_includes_line_count() {
        // GIVEN a Thinking segment whose body has a known number of lines.
        let lines: Vec<String> = vec![
            "line one".into(),
            "line two".into(),
            "line three".into(),
        ];

        // WHEN the collapsed label for that segment is produced.
        let label = thinking_collapsed_label(&lines);

        // THEN the label includes that line count AND does not include a duration.
        assert!(
            label.contains('3'),
            "label should include line count 3: {label}"
        );
        assert!(
            label.contains("line"),
            "label should mention lines: {label}"
        );
        let lower = label.to_ascii_lowercase();
        for duration_token in ["ms", "sec", "min", "hour", "duration"] {
            assert!(
                !lower.contains(duration_token),
                "label must not include duration ({duration_token}): {label}"
            );
        }
        // "s" alone is too ambiguous (matches "lines"); require no time units.
        assert!(!lower.contains("seconds") && !lower.contains("minutes"));
    }

    /// @spec chat/transcript Segment presentation: Activity collapsed label includes count and sample names
    #[test]
    fn activity_collapsed_label_includes_count_and_sample_names() {
        // GIVEN an Activity segment with multiple completed tools.
        let session = assistant_blocks(vec![
            tool_use("1", "Read", r#"{"path":"a.rs"}"#),
            tool_result("1", "Read", "ok"),
            tool_use("2", "grep", r#"{"pattern":"foo"}"#),
            tool_result("2", "grep", "match"),
            tool_use("3", "shell", r#"{"command":"ls"}"#),
            tool_result("3", "shell", "file"),
            tool_use("4", "Write", r#"{"path":"b.rs"}"#),
            tool_result("4", "Write", "written"),
        ]);
        let segs = build_transcript_segments(&session);
        let tools = activity_tools(&segs);
        assert_eq!(tools.len(), 4);

        // WHEN the collapsed label for that segment is produced.
        let label = activity_collapsed_label(tools);

        // THEN the label includes the tool count AND sample tool names.
        assert!(
            label.contains('4') && label.contains("tool"),
            "label should include tool count: {label}"
        );
        assert!(
            label.contains("Read") && label.contains("Grep") && label.contains("Shell"),
            "label should include sample humanized tool names: {label}"
        );
    }

    /// @spec chat/transcript Segment presentation: Expanded activity exposes status, summary, and truncated output
    #[test]
    fn expanded_activity_exposes_status_summary_and_truncated_output() {
        // GIVEN an expanded Activity segment with a completed tool that
        // produced multi-line output.
        let multi_line: String = (0..15).map(|i| format!("out line {i}\n")).collect();
        let session = assistant_blocks(vec![
            tool_use("1", "Read", r#"{"path":"big.txt"}"#),
            tool_result("1", "Read", &multi_line),
            tool_use("2", "grep", r#"{"pattern":"x"}"#),
            tool_result("2", "grep", "hit"),
        ]);
        let segs = build_transcript_segments(&session);
        let tools = activity_tools(&segs);

        // WHEN the segment's rows are presented.
        let rows = expanded_activity_rows(tools);

        // THEN each tool has one row showing its status and summary
        // AND truncated output is available under the row for that tool
        // AND no separate per-tool expand state is required to show it.
        assert_eq!(rows.len(), tools.len());
        for (row, tool) in rows.iter().zip(tools.iter()) {
            assert_eq!(row.status, tool.status);
            assert_eq!(row.summary, tool.summary);
            assert!(!row.status_glyph.is_empty());
            // Output is on the row itself (no nested expand flag to consult).
            assert_eq!(row.output_lines, tool.output_lines);
        }
        let first = &rows[0];
        assert_eq!(first.status, ToolRowStatus::Done);
        assert!(first.summary.contains("Read"));
        assert!(
            first.output_lines.len() > 1,
            "multi-line result should surface truncated output under the row"
        );
        assert!(
            first.output_lines.len() <= 11,
            "output should be truncated (max 10 lines + ellipsis)"
        );
    }

    // ── Segment → editor blocks ─────────────────────────────────────────

    #[test]
    fn blocks_from_segments_maps_calm_transcript_not_adjacency() {
        // Reasoning + tools + orphan-style pairing + answer become
        // Thinking / Activity / Answer — not one card per tool or "✓ done".
        let session = assistant_blocks(vec![
            ContentBlock::Reasoning("why".into()),
            tool_use("1", "Read", r#"{"path":"a.rs"}"#),
            tool_result("1", "Read", "body"),
            tool_result("orphan", "grep", "hit"),
            ContentBlock::Text("answer".into()),
        ]);
        let blocks = blocks_from_segments(&build_transcript_segments(&session));
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0].kind, BlockKind::Reasoning);
        assert_eq!(blocks[0].lines, vec!["why".to_string()]);
        assert_eq!(blocks[1].kind, BlockKind::Activity);
        assert!(blocks[1].label.contains("tool"));
        assert!(
            blocks[1].lines.iter().any(|l| l.contains("grep")),
            "orphan result should be a named activity row, not a bare done block: {:?}",
            blocks[1]
        );
        assert!(!blocks.iter().any(|b| b.label == "✓ done"));
        assert_eq!(blocks[2].kind, BlockKind::Assistant);
        assert_eq!(blocks[2].lines, vec!["answer".to_string()]);
    }

    // ── Collapse defaults ───────────────────────────────────────────────

    /// @spec chat/transcript Collapse defaults: Thinking collapses when answer follows
    #[test]
    fn thinking_collapses_when_answer_follows() {
        // GIVEN a live Thinking segment that the user has not toggled.
        let mut session = ChatSession::new("test".into());
        session.is_streaming = true;
        session.pending_reasoning = "still thinking".into();
        let segs_live = build_transcript_segments(&session);
        let mut states = Vec::new();
        sync_collapse_states(&mut states, &segs_live);
        assert_eq!(states.len(), 1);
        assert!(
            !states[0].collapsed,
            "live Thinking should start expanded"
        );
        assert!(!states[0].user_set);

        // Intermediate: reasoning committed, still streaming, NO answer yet.
        // Collapse must not fire merely because reasoning stopped receiving
        // deltas — only when Answer follows (or the turn settles).
        session.pending_reasoning.clear();
        session.messages.push(crate::chat_store::ChatMessage {
            role: Role::Assistant,
            content: vec![ContentBlock::Reasoning("still thinking".into())],
            timestamp: String::new(),
            is_priming: false,
        });
        let segs_committed = build_transcript_segments(&session);
        assert_eq!(segs_committed.len(), 1);
        assert!(
            matches!(&segs_committed[0], TranscriptSeg::Thinking { live: true, .. }),
            "committed Thinking mid-stream with no Answer should stay live: {segs_committed:?}"
        );
        sync_collapse_states(&mut states, &segs_committed);
        assert!(
            !states[0].collapsed,
            "committing Reasoning alone must not auto-collapse Thinking"
        );

        // WHEN a following Answer segment appears for the same turn.
        session.pending_text = "here is the answer".into();
        let segs = build_transcript_segments(&session);
        assert!(
            matches!(&segs[0], TranscriptSeg::Thinking { live: false, .. })
                && matches!(&segs[1], TranscriptSeg::Answer { .. }),
            "expected settled Thinking then Answer, got {segs:?}"
        );
        sync_collapse_states(&mut states, &segs);

        // THEN the Thinking segment is collapsed.
        assert!(
            states[0].collapsed,
            "Thinking should auto-collapse when Answer follows"
        );
        assert!(!states[0].user_set);
    }

    /// @spec chat/transcript Collapse defaults: User-expanded Thinking is not auto-collapsed
    #[test]
    fn user_expanded_thinking_is_not_auto_collapsed() {
        // GIVEN a Thinking segment the user has expanded.
        let mut session = ChatSession::new("test".into());
        session.is_streaming = true;
        session.pending_reasoning = "draft thought".into();
        let segs_live = build_transcript_segments(&session);
        let mut states = Vec::new();
        sync_collapse_states(&mut states, &segs_live);
        // Simulate user expanding (or re-expanding) and locking the choice.
        toggle_collapse(&mut states, 0);
        // If first-sight was expanded, toggle collapses; expand again to match
        // "user has expanded".
        if states[0].collapsed {
            toggle_collapse(&mut states, 0);
        }
        assert!(!states[0].collapsed);
        assert!(states[0].user_set);

        // WHEN a following Answer segment appears for the same turn.
        session.pending_reasoning.clear();
        session.messages.push(crate::chat_store::ChatMessage {
            role: Role::Assistant,
            content: vec![ContentBlock::Reasoning("draft thought".into())],
            timestamp: String::new(),
            is_priming: false,
        });
        session.pending_text = "answer body".into();
        let segs = build_transcript_segments(&session);
        sync_collapse_states(&mut states, &segs);

        // THEN the Thinking segment remains expanded.
        assert!(
            !states[0].collapsed,
            "user-expanded Thinking must not auto-collapse"
        );
        assert!(states[0].user_set);
    }

    /// @spec chat/transcript Collapse defaults: Settled Activity starts collapsed
    #[test]
    fn settled_activity_starts_collapsed() {
        // GIVEN a finished turn whose transcript includes an Activity segment.
        let session = assistant_blocks(vec![
            tool_use("1", "Read", r#"{"path":"a.rs"}"#),
            tool_result("1", "Read", "ok"),
            tool_use("2", "grep", r#"{"pattern":"x"}"#),
            tool_result("2", "grep", "hit"),
            ContentBlock::Text("done".into()),
        ]);
        assert!(!session.is_streaming);
        let segs = build_transcript_segments(&session);
        let activity_idx = segs
            .iter()
            .position(|s| matches!(s, TranscriptSeg::Activity { .. }))
            .expect("expected Activity segment");

        // WHEN the transcript is presented for that settled turn.
        let mut states = Vec::new();
        sync_collapse_states(&mut states, &segs);

        // THEN the Activity segment is collapsed.
        assert!(
            states[activity_idx].collapsed,
            "settled Activity should start collapsed"
        );
        assert!(!states[activity_idx].user_set);
    }

    #[test]
    fn thinking_stays_expanded_during_live_activity() {
        // GIVEN committed Reasoning + live Activity, streaming, no Answer yet.
        let mut session = assistant_blocks(vec![
            ContentBlock::Reasoning("plan the approach".into()),
            tool_use("1", "Read", r#"{"path":"a.rs"}"#),
            tool_result("1", "Read", "ok"),
            tool_use("2", "grep", r#"{"pattern":"x"}"#),
        ]);
        session.is_streaming = true;

        let segs = build_transcript_segments(&session);
        assert!(
            matches!(&segs[0], TranscriptSeg::Thinking { live: true, .. }),
            "Thinking should stay open-in-turn during tools: {segs:?}"
        );
        assert!(
            matches!(&segs[1], TranscriptSeg::Activity { live: true, .. }),
            "Activity should be live during tools: {segs:?}"
        );

        let mut states = Vec::new();
        sync_collapse_states(&mut states, &segs);

        // THEN Thinking stays expanded unless user-set.
        assert!(
            !states[0].collapsed,
            "Thinking must stay expanded while following Activity is live"
        );
        assert!(!states[0].user_set);
        assert!(
            !states[1].collapsed,
            "live Activity should start expanded"
        );
    }

    #[test]
    fn think_tools_answer_settles_thinking_collapsed() {
        // GIVEN a think → tools stream that the user has not toggled.
        let mut session = ChatSession::new("test".into());
        session.is_streaming = true;
        session.pending_reasoning = "first thought".into();
        let mut states = Vec::new();
        sync_collapse_states(&mut states, &build_transcript_segments(&session));
        assert!(!states[0].collapsed);

        // Tools flush reasoning; still no answer.
        session.pending_reasoning.clear();
        session.messages.push(crate::chat_store::ChatMessage {
            role: Role::Assistant,
            content: vec![
                ContentBlock::Reasoning("first thought".into()),
                tool_use("1", "Read", r#"{"path":"a.rs"}"#),
                tool_result("1", "Read", "body"),
            ],
            timestamp: String::new(),
            is_priming: false,
        });
        let segs_tools = build_transcript_segments(&session);
        sync_collapse_states(&mut states, &segs_tools);
        assert!(
            !states[0].collapsed,
            "Thinking should stay open through the tool phase"
        );

        // WHEN answer arrives (or the turn would complete after).
        session.pending_text = "final answer".into();
        let segs_answer = build_transcript_segments(&session);
        assert!(
            matches!(&segs_answer[0], TranscriptSeg::Thinking { live: false, .. })
                && matches!(&segs_answer[1], TranscriptSeg::Activity { .. })
                && matches!(&segs_answer[2], TranscriptSeg::Answer { .. }),
            "expected Thinking, Activity, Answer: {segs_answer:?}"
        );
        sync_collapse_states(&mut states, &segs_answer);
        assert!(
            states[0].collapsed,
            "Thinking should collapse when Answer follows"
        );

        // TurnComplete / settled: still collapsed, Activity settles too.
        session.pending_text.clear();
        session.messages[0].content.push(ContentBlock::Text("final answer".into()));
        session.is_streaming = false;
        let segs_settled = build_transcript_segments(&session);
        sync_collapse_states(&mut states, &segs_settled);
        assert!(states[0].collapsed, "settled Thinking stays collapsed");
        assert!(
            states[1].collapsed,
            "settled Activity should be collapsed"
        );
        assert!(!states[0].user_set);
    }

    /// @spec chat/transcript Collapse defaults: Priming Setup starts collapsed
    #[test]
    fn priming_setup_starts_collapsed() {
        // GIVEN a session whose first user message is the synthetic priming inject.
        let mut session = ChatSession::new("test".into());
        session.messages.push(crate::chat_store::ChatMessage {
            role: Role::User,
            content: vec![ContentBlock::Text(
                "Project conventions from AGENTS.md…\n\nDo not respond — reply with a single dot (.)"
                    .into(),
            )],
            timestamp: String::new(),
            is_priming: true,
        });
        session.messages.push(crate::chat_store::ChatMessage {
            role: Role::Assistant,
            content: vec![ContentBlock::Text(".".into())],
            timestamp: String::new(),
            is_priming: false,
        });
        session.messages.push(crate::chat_store::ChatMessage {
            role: Role::User,
            content: vec![ContentBlock::Text("real first message".into())],
            timestamp: String::new(),
            is_priming: false,
        });

        let segs = build_transcript_segments(&session);
        assert!(
            matches!(
                &segs[0],
                TranscriptSeg::User {
                    is_priming: true,
                    ..
                }
            ),
            "first segment should be priming user: {segs:?}"
        );
        assert!(
            matches!(
                &segs[2],
                TranscriptSeg::User {
                    is_priming: false,
                    ..
                }
            ),
            "real user message is not priming: {segs:?}"
        );

        // WHEN collapse state is synced for that transcript.
        let mut states = Vec::new();
        sync_collapse_states(&mut states, &segs);

        // THEN the priming Setup block is collapsed and the real user is not.
        assert!(
            states[0].collapsed,
            "priming Setup should start collapsed"
        );
        assert!(!states[0].user_set);
        assert!(
            !states[2].collapsed,
            "real user message must stay expanded"
        );

        let blocks = blocks_from_segments(&segs);
        assert!(blocks[0].is_priming);
        assert_eq!(blocks[0].label, "Setup");
        assert!(!blocks[2].is_priming);
        assert_eq!(
            priming_collapsed_label(&blocks[0].lines),
            "Setup · 3 lines"
        );
    }

    #[test]
    fn expanded_priming_stays_open_until_recollapse() {
        let mut session = ChatSession::new("test".into());
        session.messages.push(crate::chat_store::ChatMessage {
            role: Role::User,
            content: vec![ContentBlock::Text("priming body".into())],
            timestamp: String::new(),
            is_priming: true,
        });
        let segs = build_transcript_segments(&session);
        let mut states = Vec::new();
        sync_collapse_states(&mut states, &segs);
        assert!(states[0].collapsed);

        // User expands Setup.
        toggle_collapse(&mut states, 0);
        assert!(!states[0].collapsed);
        assert!(states[0].user_set);

        // Sync must not force it shut (timer owns re-hide).
        sync_collapse_states(&mut states, &segs);
        assert!(
            !states[0].collapsed,
            "user-expanded priming must not be force-collapsed by sync"
        );

        // Timer fires.
        recollapse_priming(&mut states, 0);
        assert!(states[0].collapsed);
    }
}
