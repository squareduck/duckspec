# Preferred cascade and effective model

Replace the always-`ModelRef` cascade with preferred (pin → project → global) plus catalog
availability (`EffectiveModel`), and stamp both defaults onto sessions.

## Prerequisites

- [x] @step global-default-config-and-seed

## Tasks

- [x] 1. Add `EffectiveModel`, `preferred_turn_model`, and `resolve_effective_model` in
         `crates/duckboard/src/area/interaction.rs`; remove live use of
         `builtin_default_model` as cascade floor

- [x] 2. Add `global_model_default` on `AgentSession` and stamp project + global from
         config in `refresh_model_defaults` (`crates/duckboard/src/main.rs`)

- [x] 3. Update call sites that used `resolve_turn_model` (view status,
         send/prime/recovery model field, harness for subscriptions) to preferred +
         effective resolution against the process catalog

- [x] 4. @spec harness/selection Default model resolution: A per-chat pin overrides a project default

- [x] 5. @spec harness/selection Default model resolution: A project override is preferred over the global default

- [x] 6. @spec harness/selection Default model resolution: The global default is preferred when pin and project override are unset

- [x] 7. @spec harness/selection Default model resolution: A preferred model absent from the catalog is not available

- [x] 8. @spec harness/selection Default model resolution: With no preferred model at any cascade level, the model is not available
