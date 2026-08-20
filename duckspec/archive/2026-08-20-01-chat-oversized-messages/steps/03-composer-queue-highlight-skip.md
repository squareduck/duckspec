# Composer queue highlight skip

Skip highlight on oversized composer and queue buffers, including theme `rehighlight_all`.

## Prerequisites

- [x] @step size-helper
- [x] @step oversized-user-cards

## Context

`Block.truncated` is set on oversized User cards after prefixing.
`maybe_highlight(skip =
false)` also skips when the buffer itself is oversized — do not
run it on Answer/Thinking editors or a huge answer would lose highlight. Zip
`chat_blocks`/`chat_editors` and skip only when `truncated`. Composer/queue are the
`skip=false` call sites.

Copy full is intercepted in `main::update` via `extract_copy_full_idx` (needs an iced
`Task`); `copy_full_user_text` joins the current user segment from the session.

## Tasks

- [x] 1. Wire `rehighlight_input` and `make_queue_editor` through
         `maybe_highlight(..., skip=false)` so the oversized predicate on the full buffer
         drives skip; dropping under the gate restores highlight

- [x] 2. In `crates/duckboard/src/main.rs` `rehighlight_all`, zip `chat_blocks` /
         `chat_editors` and skip when `truncated`; run `maybe_highlight` on composer and
         `queue_editor` (queue is not in that loop today)

- [x] 3. @spec chat/oversized-messages Composer and queue highlight: Oversized composer and queue skip highlight, and a composer under the gate is highlighted

- [x] 4. @spec chat/oversized-messages Composer and queue highlight: Theme rehighlight skips truncated user cards and oversized composer and queue

- [x] 5. Run focused duckboard tests for the new highlight-skip tests and fix failures
