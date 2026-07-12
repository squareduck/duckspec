# Reply oneshot parse and instruction

Bump the shared reply-suggestion oneshot to at most three `REPLY:` lines and update the
instruction to ask for most-likely, alternative, and negative freeform replies.

## Tasks

- [x] 1. In `crates/duckchat/src/reply_suggest.rs`, set `MAX_REPLIES` to 3 and update
         comments

- [x] 2. Rewrite `REPLY_SUGGEST_INSTRUCTION` to request up to three ordered `REPLY:` lines
         (most likely, alternative, negative/decline; omit if unfit) without stage-command
         preference

- [x] 3. @spec chat/default-prompts Parsed suggestion list: REPLY lines capped at three

- [x] 4. @spec chat/default-prompts Parsed suggestion list: Fewer than three REPLY lines are kept as-is

- [x] 5. @spec chat/default-prompts Oneshot request framing: Instruction asks for up to three ordered freeform REPLY lines
