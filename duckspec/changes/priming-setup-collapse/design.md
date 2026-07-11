# Priming Setup collapse - Design

Tag the synthetic first-turn user message as priming through the transcript segment
pipeline, present it as a collapsible Setup block, and re-hide after a short expand timer
without fighting Thinking/Activity collapse policy.

## Approach

```
ChatMessage { is_priming }
        │
        ▼
build_transcript_segments
  TranscriptSeg::User { is_priming }
        │
        ▼
sync_collapse_states ── first-sight collapsed for priming
        │
        ▼
blocks_from_segments ── Block { is_priming, label: "Setup" }
        │
        ▼
view_priming_user_block ── chevron header; user-card body when open
        │
  ToggleCollapse (expand)
        │
        ▼
priming_expand_gen++ + pending_priming_recollapse
        │
        ▼
main: Task sleep(PRIMING_RECOLLAPSE_SECS)
        │
        ▼
Message::RecollapsePriming { key, idx, expand_gen }
        │
        ▼
recollapse_priming (no-op if gen stale)
```

Reuse the existing index-aligned `CollapseState` model. Priming is a special case of
**User**, not a new segment kind. Timed re-hide is **not** the same as Thinking/Activity
auto-collapse: sync must not force-shut a user-expanded Setup block; only the
generation-gated timer (or a manual collapse click) does.

## Segment and block flags

`TranscriptSeg::User` and chat `Block` carry `is_priming: bool`, copied from
`ChatMessage::is_priming` at segment build. Label is `"Setup"` when priming, `"User"`
otherwise.

```rust
// agent_chat.rs
pub enum TranscriptSeg {
    User {
        lines: Vec<String>,
        is_priming: bool,
    },
    // ...
}

// text_edit::Block
pub struct Block {
    pub kind: BlockKind,
    pub label: String,
    pub lines: Vec<String>,
    pub is_priming: bool,
}
```

## Collapse policy

```rust
fn first_sight_collapsed(seg: &TranscriptSeg) -> bool {
    match seg {
        TranscriptSeg::User { is_priming: true, .. } => true,
        TranscriptSeg::Thinking { live, .. }
        | TranscriptSeg::Activity { live, .. } => !live,
        _ => false,
    }
}

// In sync_collapse_states, when !user_set:
//   priming User  → collapsed = true
//   normal User / Answer / System → collapsed = false
//   Thinking / Activity → existing settle rules
```

User expand via `toggle_collapse` sets `user_set = true` so rebuilds do not snap Setup
shut. `recollapse_priming` only sets `collapsed = true` and leaves `user_set` so the next
click still toggles cleanly.

## Presentation

`view_block` routes `BlockKind::User if block.is_priming` to `view_priming_user_block`:
muted chevron header (`Setup · N lines` when collapsed, `Setup` when open) and the
existing user-card body when expanded.

Timer constant: `PRIMING_RECOLLAPSE_SECS = 15` (mid of 10–20s product range).

## Expand timer wiring

Session fields (ephemeral, not persisted):

```rust
// AgentSession
pub priming_expand_gen: u64,
pub pending_priming_recollapse: Option<(usize /* idx */, u64 /* expand_gen */)>,
```

On `ToggleCollapse` for a priming block: bump `priming_expand_gen`; if the transition is
collapsed→expanded, set `pending_priming_recollapse`; otherwise clear it (manual
re-collapse cancels the timer).

`main` drains pending flags into `Task::perform(sleep → RecollapsePriming)`. Handler
applies collapse only when `ax.priming_expand_gen == expand_gen`.

Routing key is the existing agent session key shape: `{instance_id}/{session_id}`.

## Impact

- `crates/duckboard/src/widget/agent_chat.rs` — segments, collapse, Setup UI
- `crates/duckboard/src/widget/text_edit/state.rs` — `Block.is_priming`
- `crates/duckboard/src/area/interaction.rs` — expand scheduling on toggle
- `crates/duckboard/src/main.rs` — `RecollapsePriming` message + drain task
- No persistence schema change (`is_priming` already on messages)
- No change to priming body assembly or send path

## Decisions

- **User segment + flag, not a new kind** - keeps transcript kinds small; priming is
  already `ChatMessage::is_priming`. Alternatives: hide from transcript (loses
  inspectability), separate System bubble (wrong role).

- **Timer via generation, not `user_set` clear** - Thinking/Activity treat `user_set` as
  permanent override; Setup needs temporary expand. Generation invalidates stale sleeps
  without a cancel handle.

- **15s fixed delay** - product band was 10–20s; fixed constant keeps UX predictable and
  tests free of clock plumbing for the pure collapse path.

- **Do not auto-collapse the `.` answer** - tiny; not what blocks scroll-to-top.

## Risks

- **Scroll jump when Setup expands near the top** → accept; header click is intentional.
  Stick-to-bottom sessions are usually far below priming.

- **Stale timer after session switch** → generation + session key gate the handler; wrong
  session no-ops.
