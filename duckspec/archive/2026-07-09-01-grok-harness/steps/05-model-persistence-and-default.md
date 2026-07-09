# Model persistence and default

Thread `ModelRef` through duckboard's session and project-default persistence, and make
the default cascade resolve to grok-4.5.

## Prerequisites

- [ ] @step harness-identity-types

## Context

Today `ChatSession.selected_model` and `PersistedSession.selected_model` are
`Option<String>` (`crates/duckboard/src/chat_store.rs:77,120`), and the project default is
a `String` in `Config.model_defaults` read via `project_model_default`
(`crates/duckboard/src/config.rs`). The resolution cascade lives at
`crates/duckboard/src/area/interaction.rs:1395` and `:1279`
(`selected_model.or_else(project_model_default)`), with the default stamped in
`crates/duckboard/src/main.rs:400`. Replace the bare-string model with a `ModelRef`,
loading legacy bare strings via `ModelRef::parse_legacy`. Introduce a built-in default
`ModelRef { harness: "grok", model: "grok-4.5" }` as the final cascade fallback, replacing
today's `None` ("CLI picks").

## Tasks

- [x] 1. Change `ChatSession.selected_model` and `PersistedSession.selected_model` to
         `Option<ModelRef>`, loading legacy bare-string values via
         `ModelRef::parse_legacy`.

- [x] 2. Change the project default in `Config` to store a `ModelRef`, with the same
         legacy shim on load.

- [x] 3. Add the built-in default `ModelRef` (grok / grok-4.5) and use it as the final
         fallback in the resolution cascade.

- [x] 4. Update the send-time resolution at `interaction.rs:1395` and `:1279` and the
         default stamp at `main.rs:400` to the `ModelRef` cascade.

- [x] 5. @spec harness/selection Default model resolution: An empty cascade resolves to grok-4.5

- [x] 6. @spec harness/selection Default model resolution: A per-chat pin overrides a project default
