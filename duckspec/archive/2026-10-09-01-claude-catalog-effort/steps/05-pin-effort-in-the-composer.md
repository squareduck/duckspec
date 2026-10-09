# Pin effort in the composer

The footer pins an effort level, and a main-chat send uses it.

## Prerequisites

- [x] @step carry-effort-on-the-acp-wire
- [x] @step keep-the-last-good-claude-catalog

## Context

Claude listed models carry `ModelInfo.effort` (`default_level`, then `levels` in catalog
order). Grok and Codex list models with `effort` unset.

Later Claude refreshes emit `Message::ModelCatalogReady` again after startup (one wake at
expiry, or a five-minute retry while the slice is empty or past expiry).
`refresh_model_defaults` already runs at the start of every `update`, so a pin reconcile
there sees those replaces.

## Tasks

- [x] 1. Add `EffortPin` on `ChatSession` in `crates/duckboard/src/chat_store.rs`,
         defaulting to absent for older files.

- [x] 2. Show the effort `pick_list` between the model control and the usage readout in
         `crates/duckboard/src/widget/agent_chat.rs`, with the closed labels and a menu of
         the row's levels.

- [x] 3. Reconcile the pin in `refresh_model_defaults` in `crates/duckboard/src/main.rs`,
         and persist a clear.

- [x] 4. Set `TurnRequest.effort` from the resolved level in `send_prompt_text` and
         `recover_from_lost_session`. Leave oneshot sends unset.

- [x] 5. @spec chat/composer-footer Effort control: Effort control is shown between the model and the usage readout

- [x] 6. @spec chat/composer-footer Effort control: A pin keeps the level the chat chose

- [x] 7. @spec chat/composer-footer Effort control: Effort pin follows the preferred model when defaults are stamped

- [x] 8. @spec chat/composer-footer Effort control: Effort labels and menu follow the row's scale

- [x] 9. @spec chat/composer-footer Effort control: A main send uses the resolved effort level
