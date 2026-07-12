# Global default config and seed

Add a global main-chat default on config, seed it when unset once the process catalog has
models, and cover storage plus seed scenarios.

## Prerequisites

- [x] @step catalog-clear-on-empty-rediscovery

## Tasks

- [x] 1. Add `default_model: Option<ModelRef>` to `Config` in
         `crates/duckboard/src/config.rs` with get/set helpers and serde default `None`

- [x] 2. Implement seed helper: prefer `grok` / `grok-4.5` when in catalog, else first
         `available_models()` entry; no-op when catalog empty

- [x] 3. On `ModelCatalogReady` in `crates/duckboard/src/main.rs`, seed when
         `default_model` is `None` and persist via `config::save`

- [x] 4. @spec harness/selection Global default model setting: A configured global default is stored as an application setting

- [x] 5. @spec harness/selection Global default model setting: An unset global default is seeded from the former built-in when that model is in the catalog

- [x] 6. @spec harness/selection Global default model setting: An unset global default is seeded from the first catalog model when the former built-in is absent
