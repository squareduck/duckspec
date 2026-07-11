# Followup: live question hangs (Claude + Grok)

Post-apply user-led pass after live testing in ds-test: Claude hangs forever on
AskUserQuestion; Grok fails with a null/invalid client response. Neither path shows chips.
Wire capture pinpoints two harness-edge decode bugs.

## Scope

Implementation complete for `fast-response-question-tool` (5 steps, audit clean).
Discussed live ds-test chats, Claude 2.1 control strings, and a live Grok ACP stdio
capture of `_x.ai/ask_user_question` plus response-schema probes.

## Summary

```
| # | sev | lens | title | → next |
| --- | --- | --- | --- | --- |
| 1 | critical | soundness | Claude AskUserQuestion hangs: control `request_id` not parsed | /ds-step |
| 2 | critical | fidelity | Grok null: method `_x.ai/…` + `outcome`-tagged response | /ds-step |
```

## Issues

### 1. Claude AskUserQuestion hangs: control `request_id` not parsed - soundness/critical

**Where:** `crates/duckchat-claude-acp/src/claude/ask_user.rs`
(`parse_control_permission`); duplex control path in `claude/duplex.rs`. Live ds-test chat
`exploration-1783801450564102000` (Claude/Opus): tool pending, no chips, no `ToolResult`.

**Why:** Claude emits `control_request` with **top-level** `request_id` and
`request.subtype = "can_use_tool"`, `request.input` = questionnaire. Parser only looks for
`request_id` inside `request` → drops the line → no `control_response` → infinite wait.
Profile `tool_use` still paints “Ask user question,” so the hang looks like a stuck tool.
`--permission-prompt-tool stdio` is correct for this CLI.

**Action:** Parse top-level `request_id` (fallback nested); keep `input` as tool input.
Unit-test against the real wire shape. Response encode already matches CLI `sendResponse`
(`control_response` / success / `behavior` + optional `updatedInput`). Then retest chips →
allow/deny.

### 2. Grok null: method `_x.ai/…` + `outcome`-tagged response - fidelity/critical

**Where:** `crates/duckchat/src/acp/turn.rs` (`classify_agent_request`);
`crates/duckchat/src/acp/ask_user.rs` encode. Live ds-test chat
`exploration-1783810059860426000` (Grok): agent reports `null` instead of a proper answer
payload; no chips.

**Why:** Live ACP capture shows method **`_x.ai/ask_user_question`** (leading underscore),
params `sessionId`, `toolCallId`, `questions`, `mode`. Classifier only matches
`x.ai/ask_user_question` → Unknown → **`result: null`**. Even after matching, design’s
external tags (`Accepted` / `SkipInterview`) fail. Proved working responses:

- select:
  `{"outcome":"accepted","answers":{"<question>":"<label>"},"partial_answers":null}`

- cancel: `{"outcome":"skip_interview"}`

Allowed `outcome` variants (from reject error): `accepted`, `chat_about_this`,
`skip_interview`, `cancelled`.

**Action:** Match `_x.ai/ask_user_question` (and unprefixed alias). Encode select/cancel
with flat snake_case `outcome` tag. Add encode/classify tests from the capture. Retest
chips end-to-end.

## Open questions

None for the fix path. Optional later: whether cancel should use `cancelled` vs
`skip_interview` for product copy (both are valid variants; skip_interview was
live-proven).

## Outcome

Agreed both failures are harness-edge decode bugs, not duckboard chip layout. Plan/code
unchanged in this followup write. Suggested next: `/ds-step` then `/ds-apply` for a small
wire-fix step covering Claude parse + Grok method/encode. Not archive-ready until live
questions show chips and complete on both harnesses.
