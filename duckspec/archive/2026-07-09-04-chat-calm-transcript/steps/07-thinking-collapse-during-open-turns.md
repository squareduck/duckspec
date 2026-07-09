# Thinking collapse during open turns

Fix Thinking auto-collapse so it stays expanded through live Activity and only settles on
Answer or TurnComplete — closing the critical review finding and the test gap that let it
ship green.

## Prerequisites

- [ ] @step collapse-policy
- [ ] @step wire-transcript-into-chat-ui

## Context

Addresses findings in `reviews/01-post-implementation-calm-transcript.md`:

1. **soundness/critical** — Thinking collapses when tools start, not when Answer follows
   or the turn completes. Committed reasoning is always built with `live: false`;
   `sync_collapse_states` treats `!live` as settle, so after `flush_all_pending` on
   ToolUse, Thinking snaps shut for the whole tool phase.

2. **quality/major** — Collapse tests never cover think → tools (no Answer yet);
   `thinking_collapses_when_answer_follows` would pass for the broken policy.

3. **fidelity/minor** — Thinking body is not muted (header only).

**Where to change:** `crates/duckboard/src/widget/agent_chat.rs` —
`build_transcript_segments` (Thinking `live` fixup, parallel to Activity),
`sync_collapse_states` / `first_sight_collapsed`, unit tests in the same file's `tests`
module, and `view_thinking_block` for muted body if low-friction.

**Policy intent (design + cap):** auto-collapse Thinking only when a following Answer
appears or the turn settles — not when reasoning stops receiving deltas. While
`session.is_streaming` and no following Answer, open-turn Thinking stays expanded unless
`user_set`.

## Tasks

- [x] 1. Fix Thinking `live` (or stop using bare `!live` as the settle signal) so
         open-turn Thinking stays expanded while streaming without a following Answer —
         mirror Activity's turn-open live semantics in `build_transcript_segments` /
         `sync_collapse_states`

- [x] 2. Keep first-sight defaults: live Thinking/Activity expanded; settled/reload
         Thinking and Activity collapsed; Answer not collapsible

- [x] 3. @spec chat/transcript Collapse defaults: Thinking collapses when answer follows

- [x] 4. Unit test: committed Reasoning + live Activity, streaming, no Answer yet →
         Thinking stays expanded unless user-set

- [x] 5. Unit test: think → tools → answer (or TurnComplete) settles Thinking collapsed
         when not user-set

- [x] 6. Muted Thinking body text in `view_thinking_block` / `TextEdit` if a low-friction
         content-color style exists; otherwise leave header-only mute and note the limit
         in Outcomes

## Outcomes

Muted Thinking body deferred: `TextEdit` has no content-color / quiet-style hook, so
applying muted body text would mean a new editor API. Header stays muted; body remains
default primary until that exists.
