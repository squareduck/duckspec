# Defaults chrome and oneshot prompt caps

Hide empty-input default prompts while a main agent turn is streaming, and cap
reply-suggestion prompt bodies to the last 40 assistant lines and last 12 user lines.

## Prerequisites

- [x] @step pre-oneshot-heuristic-defaults

## Context

Line caps for oneshot framing are largely already in
`crates/duckchat/src/reply_suggest.rs` (`take_last_lines`,
`ASSISTANT_PROMPT_MAX_LINES` = 40, `USER_PROMPT_MAX_LINES` = 12, applied in
`build_reply_suggest_prompt`). Verify and keep the `@spec` tests; only fill gaps.

Streaming hide is still open: gate defaults chrome (and Tab cycle / empty-submit
use of the list) when `session.is_streaming` is true — neither list nor oneshot
loading strip. Likely touch `defaults_chrome` and/or the `agent_chat` view path
in `crates/duckboard/src/widget/agent_chat.rs` and key/submit handlers that
consult the effective list.

## Tasks

- [x] 1. Confirm (or finish) assistant/user last-N line caps in
         `crates/duckchat/src/reply_suggest.rs` `build_reply_suggest_prompt`
         (40 assistant / 12 user, truncation marker when clipped).

- [x] 2. @spec chat/default-prompts Oneshot request framing: Long assistant message is truncated to its last lines

- [x] 3. @spec chat/default-prompts Oneshot request framing: Long user message is truncated to its last lines

- [x] 4. Hide empty-input default-prompt chrome while a main turn is streaming
         (`is_streaming`): no list and no defaults loading indicator; do not arm
         empty Enter / Tab cycle from defaults during the turn.

- [x] 5. @spec chat/default-prompts Suggestion readiness: Main turn in progress hides default prompts
