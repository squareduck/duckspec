# Post-implementation — calm transcript

Reviewed `chat-calm-transcript` end-to-end (proposal → design → caps → steps → code). The
segment model and stream split are sound and well-made, but the Thinking collapse policy
collapses on tool start instead of Answer / TurnComplete — the core calm-UX trigger — so
this is not ready to accept.

## Scope

Post-implementation review of change `chat-calm-transcript`:

- `proposal.md`, `design.md`

- caps: `chat/transcript` (new), `chat/persistence` deltas

- steps 01–06 (all complete)

- code: `crates/duckboard/src/chat_store.rs`, `main.rs` stream flush,
  `area/interaction.rs` rebuild / snapshot / preamble, `widget/agent_chat.rs` segment
  builder + collapse + views, `widget/text_edit/state.rs` `BlockKind`

Deepest layer: code and its unit tests.

## Findings

### Thinking collapses when tools start, not when the answer arrives — soundness/critical

Design and cap both define Thinking auto-collapse triggers as **following Answer** or
**TurnComplete**, with live Thinking staying expanded while the turn is still open
(including alongside live Activity):

- design collapse table: auto-collapse when "first following Answer in the same turn, or
  TurnComplete"

- `caps/chat/transcript/spec.md` Collapse defaults: same triggers

- live diagram: Thinking open next to expanded Activity

Implementation in `crates/duckboard/src/widget/agent_chat.rs` does two things that break
that:

1. Committed reasoning is always built with `live: false` (`append_thinking(..., false)`).
   Only non-empty `pending_reasoning` while `is_streaming` sets `live: true`. After
   `flush_all_pending` on ToolUse, Thinking is therefore `live: false` for the whole tool
   phase.

2. `sync_collapse_states` auto-collapses Thinking when `has_following_answer || !*live` —
   so **`!live` alone** forces collapse as soon as reasoning is committed, before any
   Answer and before TurnComplete.

Typical grok turn (think → tools → answer) therefore snaps Thinking shut at the first
tool. That is the common path this change exists to calm, and it contradicts the collapse
contract.

Activity's `live` flag is adjusted at the end of `build_transcript_segments` to mean
"still open in the turn"; Thinking never gets an equivalent fixup.

The unit test `thinking_collapses_when_answer_follows` does not catch this: once pending
reasoning is flushed into a committed `Reasoning` block, `!live` already collapses
Thinking even **without** a following Answer. The test would pass for the broken policy.

**Action:** Drive Thinking auto-collapse only from "following Answer" or "turn settled"
(`!session.is_streaming` / segment no longer part of an open turn) — not from "not
currently appending ReasoningDelta". Mirror the Activity live semantics (or stop using
`!live` as a Thinking collapse trigger). Add a test that builds Reasoning + live Activity
(streaming, no Answer yet) and asserts Thinking stays expanded unless user-set.

### Collapse tests miss the think → tools window — quality/major

Related to the finding above. Collapse coverage is:

- Thinking collapses when answer follows (false confidence — see above)
- User-expanded Thinking not auto-collapsed
- Settled Activity starts collapsed

There is no scenario for "Thinking remains expanded while following Activity is live" and
no mid-stream think→tools→answer sequence. The weak test suite is why the policy bug ships
green.

**Action:** When fixing collapse, add the mid-activity stay-open case (and ideally
think→tools→answer settle) as unit tests; if the cap should name that behavior explicitly,
extend the Collapse defaults requirement first.

### Thinking body is not muted — fidelity/minor

Design's Thinking expanded sketch calls for muted body text. The header uses
`theme::text_muted()`, but the expanded body is a normal `TextEdit` on the chat surface
with default primary content color (`view_thinking_block` in `agent_chat.rs`). Secondary
role is only partially realized.

**Action:** If `TextEdit` can take a content color (or a quiet style), apply muted text
for Reasoning bodies; otherwise accept as polish and document the limit.

## What went well

- **Proposal / design / caps chain is coherent.** Harness-neutral segment model,
  view-layer grouping, group-only tool expand, Reasoning as first-class storage — right
  boundaries, no harness fork.

- **Stream split is clean.** Separate `pending_reasoning`, kind-switch flushes,
  `flush_all_pending` on tools and TurnComplete, eager snapshot folds Reasoning first then
  Text — matches design.

- **Segment builder is pure and well-tested** for contiguity and id pairing (including
  orphan named rows, no bare "✓ done").

- **Presentation helpers** (line-count Thinking label, activity count · names, quiet rows
  with truncated output) match the cap.

- **Wiring** replaces adjacency `build_chat_blocks` with `build_transcript_segments` →
  `sync_collapse_states` → `blocks_from_segments`, search labels for Thinking / Activity,
  legacy ToolUse/ToolResult kept only as non-emitted compat kinds.

- **Persistence tests** cover Reasoning round-trip, legacy load, and eager reasoning
  flush.

## Verdict

Not ready. The architecture and most of the realization are solid — this is a focused,
maintainable approach to a real UX problem, and the bulk of the code earns its keep. The
Thinking collapse policy, however, is wrong on the primary path (tools between thought and
answer), and the tests that should own that contract do not. Fix the auto-collapse trigger
(and the missing mid-activity test) before accepting; the muted-body polish is optional.
