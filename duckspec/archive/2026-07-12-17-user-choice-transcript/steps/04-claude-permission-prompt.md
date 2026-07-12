# Claude permission prompt

Fill `UserChoice.prompt` from ACP permission `toolCall.title` so Claude AskUserQuestion
questions reach live chips and settle.

## Prerequisites

- [x] @step live-question-chip-and-transcript-render

## Context

Followup #1: `classify_agent_request` hardcodes `prompt: None` for product
`session/request_permission` even though Claude sets `toolCall.title` to the question
text.

## Tasks

- [x] 1. In `crates/duckchat/src/acp/turn.rs` `classify_agent_request`, for product
         permission options extract prompt from `params.toolCall.title` (or equivalent),
         treating empty/whitespace as `None`; keep ordinary allow/reject tool permissions
         unchanged

- [x] 2. Add/adjust unit tests on Claude-shaped `session/request_permission` params so
         product options yield a non-empty prompt matching the title

- [x] 3. Re-run duckboard live-question scenarios (`live_question_prompt` / non-empty
         prompt) and any duckchat turn mid-prompt tests that assert
         `UserChoiceRequest.prompt`

- [x] 4. @spec harness/acp-client Mid-turn user choice: Permission product choice carries prompt from tool title
