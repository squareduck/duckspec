# Oneshot framing with heuristic and order

Pass the lifecycle heuristic into the reply-suggestion request as a soft hint, and rewrite
the shared instruction so multi-line replies order as continue → alternatives → negative.

## Prerequisites

- [x] @step reply-parse-and-provider-oneshot

## Context

Extends the existing duckchat oneshot from step 01. `ReplySuggestionRequest` currently
omits the heuristic (docs still say the caller merges after parse — that is obsolete).
Update `REPLY_SUGGEST_INSTRUCTION` and `build_reply_suggest_prompt` in
`crates/duckchat/src/reply_suggest.rs`; field on `ReplySuggestionRequest` in `request.rs`;
pass `ax.obvious_command` when spawning in `crates/duckboard/src/main.rs`. Keep
`grok-composer-2.5-fast` / Haiku model picks.

## Tasks

- [x] 1. Add optional `lifecycle_heuristic: Option<String>` (or equivalent) on
         `ReplySuggestionRequest`; update docs that claimed the heuristic is never carried

- [x] 2. Rewrite `REPLY_SUGGEST_INSTRUCTION` for 1–3 `REPLY:` lines ordered: first = most
         obvious continue, middle = alternatives, last = negative when appropriate;
         heuristic is a soft hint only

- [x] 3. Include the heuristic in `build_reply_suggest_prompt` when present; unit-test
         request/prompt construction

- [x] 4. @spec chat/default-prompts Oneshot request framing: Heuristic is included in the request when present

- [x] 5. @spec chat/default-prompts Oneshot request framing: Ordering guidance is present in the instruction

- [x] 6. @spec chat/default-prompts Oneshot request framing: Empty assistant yields empty list without a model call

- [x] 7. Pass the session lifecycle heuristic into the oneshot spawn path in duckboard
         (`TurnComplete` → `reply_suggestions`)
