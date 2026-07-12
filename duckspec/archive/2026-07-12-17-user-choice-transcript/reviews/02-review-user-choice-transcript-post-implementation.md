# Review: user-choice transcript post-implementation

Post-implement review of host Q→A chips, Claude prompt wiring, `Question: ` prefix, and
AskUserQuestion Activity suppression. Solid core; main gap is caps lagging followup
behavior.

## Scope

`proposal.md`, `design.md`, `caps/chat/fast-response` + `persistence` deltas, steps 01–06,
followup `01-followup-…`, and touched code in `duckboard`, `duckchat` ACP turn,
`duckchat-claude-acp`.

## Summary

```
| # | sev | lens | title | → next |
| --- | --- | --- | --- | --- |
| 1 | major | fidelity | Specs omit followup contracts | /ds-spec |
| 2 | minor | quality | Prefix formatter only matches exact `Question: ` | /ds-step |
| 3 | minor | quality | Settle marks dirty; no immediate materialize on chip pick | ignore |
```

## Findings

### 1. Specs omit followup contracts - fidelity/major

**Where:** `duckspec/changes/user-choice-transcript/caps/chat/fast-response/spec.delta.md`
(and absences for harness/Activity); code: `permission_choice_prompt` in
`crates/duckchat/src/acp/turn.rs`, `format_user_choice_question_text` in
`crates/duckboard/src/fast_response.rs`, `is_host_choice_tool_name` in
`crates/duckboard/src/widget/agent_chat.rs`.

**Why:** Caps still describe “question text” without the `Question: ` prefix, don’t pin
permission `toolCall.title` → prompt, and don’t require omitting AskUserQuestion from
Activity. Freezing archive-as-is leaves the durable contract behind the product and the
followup log.

**Action:** Small `/ds-spec` delta pass: prefix on store/display, permission-title prompt
source (or harness/acp-client), suppress host-choice tools from Activity.

### 2. Prefix formatter only matches exact `Question: ` - quality/minor

**Where:** `crates/duckboard/src/fast_response.rs` — `format_user_choice_question_text` /
`USER_CHOICE_QUESTION_PREFIX`.

**Why:** `"Question:foo"` or case variants get a second prefix. Low frequency if agents
send bare questions; cheap to harden.

**Action:** Optional polish in a micro-step: case-insensitive prefix strip/normalize
before re-prefix.

### 3. Settle marks dirty; no immediate materialize on chip pick - quality/minor

**Where:** `settle_user_choice_transcript` sets `chat_ui_dirty`; `ActivateFastResponse`
does not call `materialize_chat_ui` (`crates/duckboard/src/area/interaction.rs`).

**Why:** While the turn is still streaming, painted Q/A wait for StreamTick (or turn end)
rather than the click itself. Usually fine if ticks are frequent; possible brief lag.

**Action:** Accept as-is unless you see stuck UI; then force materialize after settle on
activation/freeform paths.

## Verdict

**Ship-worthy with a fidelity follow-through.** Implementation matches design intent:
in-band wire, host Q→A blocks, Claude title→prompt, prefix, Activity suppress. Tests and
audit are green. Biggest lasting debt is **caps not updated after the followup fixes** —
address with `/ds-spec` before archive if you want the frozen contract to match reality.
Items 2–3 are optional polish.
