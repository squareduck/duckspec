# Demote ds-verify from auto-suggest

Stop the reply-suggestion oneshot from advertising `/ds-verify` as a normal workflow next
step, while leaving the skill installed for rare diagnostics.

## Context

`/ds-verify` still appears in empty-input defaults because discovered slash commands
(including `ds-verify`) are primed into the reply-suggest prompt and the instruction
prefers skill/stage slash commands. Filter and instruction copy live in
`crates/duckchat/src/reply_suggest.rs` (request filled from `crates/duckboard/src/main.rs`
`available_commands`).

## Tasks

- [x] 1. Update `REPLY_SUGGEST_INSTRUCTION` so preferred workflow slash commands are the
         main-flow stages only; explicitly do not suggest `/ds-verify` (side diagnostic,
         not usual lifecycle).

- [x] 2. When building the oneshot prompt (and/or when assigning `available_commands` on
         the request), exclude `ds-verify` / `/ds-verify` from the available-commands
         priming list so the model is not nudged by discovery alone.

- [x] 3. Optionally drop parsed `REPLY:` lines that are exactly `/ds-verify` or
         `ds-verify` so a model that invents it still cannot arm the chrome (document the
         choice in a brief comment if you filter at parse time).

- [x] 4. Add or extend unit tests in `reply_suggest.rs` for the instruction guidance and
         for command filtering (and parse filter if implemented).

- [x] 5. Leave the verify skill/template and title-hint in place; do not delete
         `/ds-verify` as a command.
