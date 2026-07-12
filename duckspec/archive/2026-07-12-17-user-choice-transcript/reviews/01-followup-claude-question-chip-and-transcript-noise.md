# Followup: Claude question chip and transcript noise

Post-implementation try with Claude: option chips show, but the question chip does not;
AskUserQuestion also pollutes Activity. Plus a presentation tweak for settled questions.

## Scope

Live Claude AskUserQuestion in duckboard; `chat/fast-response` settle path; ACP
`session/request_permission` classify; Claude `tool_call` updates for AskUserQuestion.

## Summary

```
| # | sev | lens | title | → next |
| --- | --- | --- | --- | --- |
| 1 | major | soundness | Claude user-choice prompt is always None | /ds-step |
| 2 | minor | quality | Prefix settled question with `Question: ` | /ds-step |
| 3 | major | quality | Suppress AskUserQuestion tool cards | /ds-step |
```

## Issues

### 1. Claude user-choice prompt is always None - soundness/major

**Where:** `crates/duckchat/src/acp/turn.rs` — `classify_agent_request` for product
`session/request_permission` sets `prompt: None` even though Claude puts the question on
`toolCall.title` (see `permission_request_params` in
`crates/duckchat-claude-acp/src/claude/ask_user.rs`).

**Why:** Live question chip and settle only work when prompt is non-empty; Claude never
shows or stores the question. Grok’s ask-user method path already decodes prompt
correctly.

**Action:** Parse `toolCall.title` into `UserChoice.prompt` on the permission product
path; add/adjust tests that expect a non-empty prompt from Claude-shaped permission
params.

### 2. Prefix settled question with `Question: ` - quality/minor

**Where:** Host settle commit and settled chip render (`settle_user_choice_transcript` /
transcript question chips in duckboard).

**Why:** Settled chips should read clearly as questions in history. Prefix should be in
storage so reload matches; live chrome can share the same formatter so live and history
match.

**Action:** When committing question text, store `Question: <raw>` (idempotent if already
prefixed); settled render uses the stored body.

### 3. Suppress AskUserQuestion tool cards - quality/major

**Where:** Claude bridge emits `tool_call` updates titled AskUserQuestion
(`tool_call_update` in `crates/duckchat-claude-acp`); duckboard Activity groups show “Ask
user question” rows beside the choice chips.

**Why:** Duplicate, noisy UI once chips and the question chip own the interaction surface.

**Action:** Do not surface AskUserQuestion (and equivalent labels) as host tool Activity —
skip emit and/or filter at transcript build — without breaking the permission/choice wire.

## Outcome

Three followups before archive: fix Claude prompt plumbing, question label prefix on
store/render, and suppress AskUserQuestion tool-card noise. Spec delta likely small
(permission prompt source, store format, omit question tools from Activity); then
`/ds-step` and apply.
