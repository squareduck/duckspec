# Missing send gate and closed label

Block main-chat turns when the effective model is not available, and show `Missing` on the
closed model control.

## Prerequisites

- [x] @step preferred-cascade-and-effective-model

## Tasks

- [x] 1. Gate `send_prompt_text` and other main-chat turn starts in
         `crates/duckboard/src/area/interaction.rs` so non-`Available` effective models
         no-op (no message, no stream, no substitute)

- [x] 2. Build closed model choice with label/closed_label `Missing` when effective is not
         available (`agent_chat` + interaction status path); meter gets no context window

- [x] 3. @spec harness/selection Send requires an available model: A turn does not start when the preferred model is not available

- [x] 4. @spec chat/composer-footer Missing closed model label: Closed label is Missing when the effective model is not available
