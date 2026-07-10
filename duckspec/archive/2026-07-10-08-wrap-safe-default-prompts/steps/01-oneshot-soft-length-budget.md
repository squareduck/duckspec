# Oneshot soft length budget

Add a named 100-character soft budget to the shared reply-suggestion instruction and prove
parse never truncates over-budget text.

## Tasks

- [x] 1. Add `REPLY_SUGGEST_MAX_CHARS: usize = 100` in
         `crates/duckchat/src/reply_suggest.rs`

- [x] 2. Soft-ask each REPLY text ≤ that budget in `REPLY_SUGGEST_INSTRUCTION` (no
         parse-time truncate)

- [x] 3. @spec chat/default-prompts Oneshot request framing: Length guidance is present in the instruction

- [x] 4. @spec chat/default-prompts Parsed suggestion list: Reply longer than 100 characters is preserved in full
