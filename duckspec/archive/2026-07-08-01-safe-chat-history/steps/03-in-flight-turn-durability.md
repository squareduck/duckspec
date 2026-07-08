# In-flight turn durability

Persist a scope's sessions before any state mutation, and flush the streaming session
periodically during a turn so an abrupt end loses at most a bounded tail.

## Prerequisites

- [ ] @step non-destructive-scope-migration

## Context

Step 02 landed the merge primitives with slightly wider signatures than the design
sketched — carry these forward:

- `interaction::merge_sessions(into: &mut InteractionState, incoming: Vec<AgentSession>,
  scope_label: &str)` — takes a `scope_label` (needed to re-run
  `reconcile_display_names`), re-sorts newest-first, and preserves the active session by
  id. `into.instance_id` is untouched.
- `chat_store::merge_scope(from, to, project_root)` is the on-disk counterpart and
  replaced `rename_scope` in both promote paths.

Both `promote_exploration` (`area/change.rs`) and `promote_idea_exploration`
(`main.rs`) now merge-instead-of-clobber; task 1's `flush_sessions` call goes at the very
top of each, before the existing `interactions.remove`.

## Tasks

- [x] 1. Add a `flush_sessions(ix, project_root)` helper and call it at the top of
         `promote_exploration` / `promote_idea_exploration`, before any
         `interactions.remove` / `insert` / `merge_sessions`.

- [x] 2. Track a dirty flag for the active session; mark it dirty on the message-mutating
         `AgentEvent`s (`ContentDelta` flush, `ToolUse`, `ToolResult`).

- [x] 3. Drive a coalesced ~1s flush of the dirty session while streaming, and force an
         immediate flush at turn boundaries, before any mutate, and on app quit — confirm
         the iced app loop exposes a shutdown / `window::close` hook for the quit flush,
         and use the debounce interval plus turn-boundary saves as the bound if it does
         not.

- [x] 4. @spec chat/persistence In-flight turn durability: An in-flight turn survives a promotion

- [x] 5. @spec chat/persistence In-flight turn durability: Streamed messages are persisted before turn completion

## Outcomes

- **The iced 0.14 loop does expose a clean quit hook**, so task 3's fallback wasn't
  needed: `.exit_on_close_request(false)` on the app builder plus an
  `iced::window::close_requests()` subscription lets `Message::WindowCloseRequested`
  flush every session and then `iced::window::close(id)`. The design's "clean quit flush
  depends on a shutdown hook" risk is closed. Turn-boundary and ~1s debounced saves still
  bound a hard-crash (non-close) exit.
- **Eager flush persists a snapshot, not the live session:** `persist_session_snapshot`
  folds in-flight `pending_text` into a trailing assistant message on a clone, so streamed
  prose survives a crash without fragmenting the committed `messages` list (the real
  `flush_pending_text` still runs at the next `ToolUse`/`TurnComplete`).
- The two `@spec` tests live in `chat_store.rs`'s test module rather than
  `change.rs`/`interaction.rs`, to reuse the existing `HOME_LOCK`/`with_home` harness and
  avoid a cross-module race on the process-global `HOME` env var.
