# Multi-question requestUserInput

Answer all questions in a structured user-input request (1–3), not only the first, so App
Server questionnaires complete with a full answers map.

## Prerequisites

- [x] @step permission-grant-auto-approval-shapes

## Context

From review finding 3
(`reviews/02-review-post-implementation-review-of-openai-codex-harness.md`).

Schema: `questions[]` with required `id` / `header` / `question`; response maps question
id → `{ "answers": ["…"] }`. Implement sequential host choices per question and merge into
one `answers` object. Cancel on any question cancels the whole questionnaire.

## Tasks

- [x] 1. Decode the full `questions[]` (id, text, options) in
         `crates/duckchat-codex-acp/src/codex/ask_user.rs`

- [x] 2. In `service_parent_choice` / prompt orchestration
         (`crates/duckchat-codex-acp/src/agent.rs`): sequential host choices per question;
         merge into one `answers` map keyed by question id

- [x] 3. Preserve cancel semantics (any cancel → cancelled for the whole questionnaire)

- [x] 4. Unit tests: two-question accept; multi-question cancel

- [x] 5. @spec harness/openai-codex Mid-turn structured questions: A structured user-input request surfaces a host user choice

- [x] 6. @spec harness/openai-codex Mid-turn structured questions: Host selection completes with accepted answers

- [x] 7. @spec harness/openai-codex Mid-turn structured questions: Host custom freeform completes with accepted free-text answers

- [x] 8. @spec harness/openai-codex Mid-turn structured questions: Host cancel completes without accepting the questionnaire
