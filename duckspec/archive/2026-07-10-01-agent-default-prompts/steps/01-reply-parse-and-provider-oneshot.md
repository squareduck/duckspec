# Reply parse and provider oneshot

Add shared `REPLY:` parsing and a Provider oneshot sibling of `title_summary` on both
harnesses, covering the parsed-suggestion-list scenarios.

## Tasks

- [x] 1. Add `ReplySuggestionRequest` and `Provider::reply_suggestions` in duckchat
         (`request.rs`, `provider.rs`, re-export from `lib.rs`)

- [x] 2. Implement shared `parse_replies` (cap 3, order-preserving, unknown slash kept)
         and unit-test the parse scenarios

- [x] 3. @spec chat/default-prompts Parsed suggestion list: REPLY lines extracted in order and capped at three

- [x] 4. @spec chat/default-prompts Parsed suggestion list: No matching lines yields an empty list

- [x] 5. @spec chat/default-prompts Parsed suggestion list: Unknown slash text is preserved

- [x] 6. Implement `reply_suggestions` on ClaudeCodeProvider and GrokProvider using the
         same cheap-model pick as title summary; empty assistant message short-circuits to
         `Ok(vec![])` without a model call
