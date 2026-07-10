# Composer chrome and auto-grow

Render loading while pending, the effective list only when ready and empty, and restore
composer auto-grow without height jumps while the user is already typing.

## Prerequisites

- [x] @step suggestion-readiness-state

## Context

Addresses the critical auto-grow and minor ghost-stack findings in
`reviews/01-post-implementation-review.md`, plus the height-jump-while-typing bug: today
`agent_chat.rs` uses `stack![prompt_list, input]` (list is iced base layer → caps grow)
and reserves list height even when `input` is non-empty. Prefer a column sibling layout
(input grows with content; list or loading below only when input empty). Do not reserve
vertical space for suggestions when the input has text. Model pick stays
`grok-composer-2.5-fast` / Haiku — latency is cold ACP spawn, out of this step’s scope.

**From step 06:** session field is `default_prompts_pending: bool` (true = pending). Pure
helpers `empty_submit_text(pending, …)` and `can_cycle_defaults` already gate keyboard;
view must thread `ax.default_prompts_pending` into chrome (loading vs list).

## Tasks

- [x] 1. Thread pending/ready + effective list into `agent_chat::view`; when input empty
         and pending, show a loading indicator and no prompt list

- [x] 2. When input empty and ready, show the effective list (active marker; cycle order
         as today or design strip); paint non-active rows fainter than the active entry;
         hide all suggestion chrome when input is non-empty — no height reserve

- [x] 3. Fix layout so `TextEdit` auto-grow works with defaults present (column under
         input, or stack with input as base + outer height max); multi-line type/paste
         must grow up to `CHAT_INPUT_MAX_ROWS`

- [x] 4. @spec chat/default-prompts Suggestion readiness: Pending hides list and shows loading

- [x] 5. Drop obsolete single-placeholder / merge-era chrome if any remains; smoke-check
         empty Enter + Tab only when ready
