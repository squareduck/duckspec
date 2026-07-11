# Post-implementation review: turn answer replace

Reviewed proposal → design → caps → steps → code for answer-draft replace, thrash budget,
Thinking fade, and style emit-once. Core behavior matches intent and is tested; only small
cancel-parity and process nits remain.

## Scope

`proposal.md`, `design.md`, `caps/chat/stream-ui` + `transcript` deltas, all three steps,
and code under `crates/duckboard` (`interaction`, `chat_store`, `main`, `agent_chat`,
`text_edit/render`) plus `crates/duckspec/content/schemas/style.md`. Post-implementation;
audit clean.

## Summary

```
| # | sev | lens | title | → next |
| --- | --- | --- | --- | --- |
| 1 | minor | soundness | Thrash cancel skips CancelPressed priming cleanup | /ds-step |
| 2 | minor | quality | Thinking fade marked done without in-app light/dark check | ignore |
```

## Findings

### 1. Thrash cancel skips CancelPressed priming cleanup - soundness/minor

**Where:** `crates/duckboard/src/main.rs` ContentDelta thrash path vs `CancelPressed` in
`crates/duckboard/src/area/interaction.rs`

**Why:** User cancel clears `priming_in_flight` / `pending_followup_prompt`; thrash only
cancels the agent handle. If thrash ever fired during priming, TurnComplete could still
dispatch a staged follow-up. Unlikely (priming is a short “.”), but the two cancel paths
diverge and that inconsistency can surprise later.

**Action:** Mirror CancelPressed cleanup on thrash trip (or share one helper) if cancel
parity is wanted before archive; safe to ignore if priming thrash is accepted as
impossible.

### 2. Thinking fade marked done without in-app light/dark check - quality/minor

**Where:** step 03 task 4; `view_thinking_block` + `TextEdit::base_color(text_secondary)`
in `crates/duckboard/src/widget/agent_chat.rs` / `text_edit/render.rs`

**Why:** Implementation matches design; the scenario is `manual:` and was checked without
a recorded light/dark eyeball. Low lasting cost if secondary ink is wrong in one
theme—easy to tweak later.

**Action:** Optional quick visual pass in both themes; no spec/step change required.

## Verdict

**Accept / archive-ready.** Draft replace, thrash budget (keep last complete draft,
notice, drop further deltas, tool reset), transcript one-answer span, and style emit-once
are coherent end-to-end. Keeping the last full draft on the third replace is a sound
improvement over the design’s “append then trip” sketch. Remaining items are minor parity
and manual polish, not blockers.
