# Oversized user cards

Truncate User display blocks on materialize, skip highlight, and add a copy-full control
that writes the current stored user body.

## Prerequisites

- [x] @step size-helper

## Context

`is_oversized`, `display_prefix`, and `maybe_highlight` already live in
`crates/duckboard/src/area/interaction.rs` (`OVERSIZED_LINE_CAP` 200, `OVERSIZED_CHAR_CAP`
16_384). The last two currently have `#[allow(dead_code)]` until this step wires them.

## Tasks

- [x] 1. Add `truncated: bool` to `Block` in
         `crates/duckboard/src/widget/text_edit/state.rs` (default false). In
         `rebuild_chat_editor`, when a User segment's full body is oversized, set
         `Block.lines` to `display_prefix` and `truncated = true`; call
         `maybe_highlight(..., skip=true)` instead of `make_highlighted_editor` /
         `refresh_editor_in_place` highlight for that block

- [x] 2. In `crates/duckboard/src/widget/agent_chat.rs`, add `Msg::CopyFull(usize)` and a
         footer on truncated User cards that invokes it. Handle the message so the
         clipboard receives the current matching user segment body from the session (join
         at click time)

- [x] 3. @spec chat/oversized-messages User display prefix: Oversized user card is an unhighlighted prefix

- [x] 4. @spec chat/oversized-messages User display prefix: Copy full writes the stored user body

- [x] 5. Run focused duckboard tests for the new user-card tests and fix failures
