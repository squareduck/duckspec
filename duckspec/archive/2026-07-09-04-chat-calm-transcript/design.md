# Calm chat transcript — Design

Split streaming buffers and persisted content so reasoning never mixes with answers, then
rebuild the transcript as contiguous Thinking / Activity / Answer segments with settled
collapse defaults — all from neutral `AgentEvent`s, no harness forks.

## Approach

duckchat already emits thinking, answer text, and tools on separate channels. The bug is
downstream: duckboard co-mingles reasoning into `pending_text`, flushes tools as one-card-
per-call blocks, and pairs results only with the adjacent item. This design keeps the
harness layer unchanged and rewires the session model + view pipeline.

```
AgentEvent (duckchat / duckboard agent)
       │
       │  ReasoningDelta / ContentDelta / ToolUse / ToolResult / TurnComplete
       ▼
┌──────────────────────────────────────────────────────────┐
│ Stream flush  (main.rs event handler)                    │
│   pending_reasoning  │  pending_text  │  open tool msgs  │
│   flush on kind switch · TurnComplete · persist snapshot │
└────────────────────────────┬─────────────────────────────┘
                             │
                             ▼
┌──────────────────────────────────────────────────────────┐
│ ChatSession.messages                                     │
│   ContentBlock::Reasoning | Text | ToolUse | ToolResult  │
└────────────────────────────┬─────────────────────────────┘
                             │
                             ▼
┌──────────────────────────────────────────────────────────┐
│ build_transcript_segments(&session) → Vec<TranscriptSeg> │
│   contiguity + id-based tool pairing (pure, unit-tested) │
└────────────────────────────┬─────────────────────────────┘
                             │
                             ▼
┌──────────────────────────────────────────────────────────┐
│ view_segment + collapse policy                           │
│   Thinking (muted) · Activity (quiet rows) · Answer      │
└──────────────────────────────────────────────────────────┘
```

Live vs settled (same segment model, different defaults):

```
LIVE                                      SETTLED
────                                      ──────
[◉ Thinking ···  open]                    ▸ Thinking · 12 lines
[ Activity expanded:                      ▸ 4 tools · Read, grep, shell
    ✓ Read …
    ● running … ]
[answer streaming …]                      answer prose (primary)
```

## Session model and persistence

Extend `ContentBlock` so reasoning is durable and never stored as `Text`. Add a second
in-memory buffer so streaming thoughts and answers do not share a string.

```rust
// crates/duckboard/src/chat_store.rs

pub enum ContentBlock {
    Text(String),
    Reasoning(String),
    ToolUse { id: String, name: String, input: String },
    ToolResult { id: String, name: String, output: String },
}

pub struct ChatSession {
    // …
    pub pending_text: String,
    /// Streaming reasoning, distinct from answer prose. Not persisted as a
    /// field — folded into `ContentBlock::Reasoning` on flush / snapshot.
    pub pending_reasoning: String,
    // …
}
```

Serde: new enum variant is additive. Old session JSON without `Reasoning` loads unchanged.
No version field required.

Eager persist (`persist_session_snapshot`) today folds only `pending_text` into a trailing
`Text` message. It must also fold `pending_reasoning` into a trailing `Reasoning` message
(order: flush reasoning first if both are non-empty when snapshotting mid-kind — normally
only one buffer is active).

History preamble (`build_history_preamble`) gains a branch for reasoning so fresh-session
replays still carry “the agent thought …” context without dumping full thought text if we
choose a short form:

```rust
ContentBlock::Reasoning(t) => {
    // Include body: agents benefit from prior thought on non-resume paths.
    out.push_str("[Assistant reasoning]\n");
    out.push_str(t);
    out.push_str("\n\n");
}
```

## Stream flush path

`main.rs` currently pushes both `ContentDelta` and `ReasoningDelta` into `pending_text`
and flushes that buffer before a tool use. Replace with kind-aware flushes:

```rust
// crates/duckboard/src/main.rs  (sketch)

fn flush_pending_reasoning(session: &mut ChatSession) { /* → ContentBlock::Reasoning */ }
fn flush_pending_text(session: &mut ChatSession) { /* → ContentBlock::Text (existing) */ }
fn flush_all_pending(session: &mut ChatSession) {
    flush_pending_reasoning(session);
    flush_pending_text(session);
}

// on ReasoningDelta:
//   flush_pending_text(session);  // kind switch away from answer
//   session.pending_reasoning.push_str(&text);

// on ContentDelta:
//   flush_pending_reasoning(session);
//   session.pending_text.push_str(&text);

// on ToolUse:
//   flush_all_pending(session);
//   push ToolUse message (unchanged message granularity)

// on ToolResult:
//   push ToolResult message

// on TurnComplete:
//   flush_all_pending(session);
```

Tool message granularity stays one `ChatMessage` per use/result (matches today). Grouping
is a view concern so pairing and activity runs can re-group without migration.

## Segment builder

Replace adjacency-only `build_chat_blocks` with a pure segment builder. Call sites
(`rebuild_chat_editor`, search labels) move to segments or a thin adapter that still
yields `Block`s for the existing editor stack.

```rust
// crates/duckboard/src/widget/agent_chat.rs  (or transcript.rs)

pub enum TranscriptSeg {
    User { lines: Vec<String> },
    System { lines: Vec<String> },
    Thinking {
        lines: Vec<String>,
        /// True while this segment is still receiving ReasoningDelta
        /// (session.is_streaming && pending_reasoning feeds it).
        live: bool,
    },
    Answer {
        lines: Vec<String>,
        live: bool,
    },
    Activity {
        tools: Vec<ToolRow>,
        live: bool,
    },
}

pub struct ToolRow {
    pub id: String,
    pub summary: String,       // format_tool_summary
    pub output_lines: Vec<String>, // truncate_output; empty if still running
    pub status: ToolRowStatus,
}

pub enum ToolRowStatus {
    Running,
    Done,
    Error, // optional: detect from empty/error-shaped output later
}

pub fn build_transcript_segments(session: &ChatSession) -> Vec<TranscriptSeg> {
    todo!("flatten messages; coalesce by kind; pair tools by id")
}
```

Contiguity rules:

```
walk flattened ContentBlocks (assistant stream order):
  Reasoning*  → one Thinking segment (grow while contiguous)
  Text*       → one Answer segment
  (ToolUse|ToolResult)* → one Activity segment

User / System messages → their own segments (unchanged visually)

pending_reasoning (if streaming) → open or append live Thinking
pending_text (if streaming)      → open or append live Answer
in-flight ToolUse without result → ToolRowStatus::Running inside open Activity
```

Tool pairing: maintain `HashMap<id, ToolRow>` within the current activity run. A
`ToolResult` updates the matching row by id even if other results/uses interleaved inside
the run. Results whose id never appeared are attached as a done row with summary from
`name` (not the bare label `✓ done`). When a non-tool block appears, the activity segment
closes.

Settled group summary label (for collapse header):

```rust
fn activity_summary(tools: &[ToolRow]) -> String {
    // e.g. "4 tools · Read, grep, shell"
    todo!()
}
```

Thinking collapsed label: `Thinking` or `Thinking · N lines` from `lines.len()` — no
timestamps.

## Collapse policy

`AgentSession.chat_collapsed: Vec<bool>` stays index-aligned with segments. Defaults when
a segment **first appears** (new index ≥ old_len), then auto-updates for policy triggers
without fighting a user override.

```
kind        live?     default on first sight     auto-collapse when
──────────  ────────  ─────────────────────────  ──────────────────────────
Thinking    true      expanded                   first following Answer in
                                                 the same turn, or TurnComplete
Thinking    false     collapsed                  (reload / already settled)
Activity    true      expanded                   next Answer starts, or
                                                 TurnComplete
Activity    false     collapsed                  —
Answer      *         n/a (not collapsible)      —
User/Sys    *         n/a                        —
```

User toggle on a segment sets an override bit (or simply: once toggled, skip further
auto-collapse for that index). Implementation sketch: parallel
`chat_collapse_user_set: Vec<bool>` or store `CollapseState { collapsed, user_set }`.

`rebuild_chat_editor` today forces new tool blocks collapsed. Replace that with the table
above keyed on segment kind + `live`.

## View and theme

Three visual treatments; reuse collapsible chevron and tool-card frame only where they
earn their keep.

```
Thinking (expanded)              Thinking (collapsed)
┌──────────────────────┐         ▸ Thinking · 12 lines
│ Thinking ···         │           muted one-liner, full width
│ muted body text…     │
└──────────────────────┘

Activity (live)                  Activity (settled collapsed)
┌──────────────────────┐         ▸ 4 tools · Read, grep, shell
│  ✓ Read agent_chat   │
│  ✓ grep Reasoning    │
│  ● shell `ds status` │  ← current
│    (truncated out…)  │
└──────────────────────┘

Answer — plain assistant prose, no card (unchanged)
```

v1 expand depth for tools (**resolved**):

- Group is the only collapse unit.

- When expanded, every tool is a quiet row: status glyph + summary; truncated output
  inline under the row when non-empty.

- No nested per-tool expand state.

Theme hooks: muted thinking text color; activity header uses existing tool-card header
styles or a quieter variant; drop full bordered card chrome for individual tools.

`view_block` / `view_tool_block` grow a `view_thinking_segment` and
`view_activity_segment` path. Search / selection context that labels blocks by kind gains
`Thinking` / `Activity` names (`main.rs` chat search labels).

## Block kinds and editor stack

Minimal path: map segments into existing `Block` + `BlockKind` so `EditorState` per
segment still works.

```rust
// crates/duckboard/src/widget/text_edit/state.rs

pub enum BlockKind {
    User,
    Assistant,   // Answer
    Reasoning,   // Thinking body when expanded
    ToolUse,     // Activity: header uses summary label; lines = joined row dump
    ToolResult,  // retire from builder output (keep variant for compat if needed)
    System,
}
```

Alternatively introduce `BlockKind::Activity` and stop emitting `ToolResult` blocks from
the builder entirely (orphans no longer surface as blocks). Prefer one `Activity` kind so
collapse and styling are not overloaded on `ToolUse`.

`ToolResult` may remain on `ContentBlock` (persistence) without a dedicated view kind.

## Decisions

- **Harness-neutral transcript** — all presentation logic in duckboard from neutral
  events. Alternatives: grok-only UI branch (rejected: proposal forbids harness forks;
  Claude still benefits from activity grouping).

- **Grouping in the view layer** — keep one message per tool use/result; segment builder
  groups. Alternatives: coalesce tools into one persisted message (rejected: harder
  streaming updates and migrations).

- **Group-only tool expand (v1)** — expanded activity shows quiet rows with inline
  truncated output; no nested per-tool expand. Alternatives: nested expand (deferred: more
  collapse state and layout edge cases for little v1 gain).

- **Thinking labels by line count** — collapsed `Thinking · N lines`; no start/end
  timestamps persisted. Alternatives: duration labels with persisted clocks (rejected:
  cosmetic cost / schema noise); live-only timer (optional later, not required for settled
  UX).

- **Reasoning persisted as its own block** — not ephemeral. Alternatives: drop thoughts
  after answer (rejected: reload loses “why”; history preamble weaker).

## Risks

- **Segment index churn mid-stream shifts collapse flags** → rebuild conservatively: match
  stable keys (e.g. last segment kind + tool ids) when possible; only apply defaults to
  truly new trailing segments; user_set bits prevent auto-collapse after manual expand.

- **Old sessions with mixed thought+answer in `Text`** → leave as Answer prose; no
  heuristic split. Only new turns get clean Thinking segments.

- **Large tool outputs still bloat expanded groups** → keep `truncate_output` (existing
  max lines); group collapse remains the primary quieting mechanism.

## Open questions

None — resolved in design:

1. Tool expand depth → group-only with inline truncated output.
2. Thinking labels → line count only; no persisted timestamps.
