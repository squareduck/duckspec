# Oneshot model resolution and worker wiring

Add global `chat.oneshot_models` config, resolve preferred oneshot models against the
catalog, and inject the resolved preference into the worker oneshot path.

## Prerequisites

- [x] @step process-model-catalog

## Tasks

- [x] 1. Extend `ChatConfig` with `oneshot_models: HashMap<String, String>` (harness →
         model id) in `crates/duckboard/src/config.rs`; load/save/round-trip

- [x] 2. Implement `resolve_oneshot_model` (configured if in catalog → string-match
         default → first catalog model) with per-harness match needles

- [x] 3. Thread preferred oneshot into `Provider::open_oneshot_runtime` / `spawn_worker`
         and resolve at harness dispatch in `drive_provider` (remove hard-coded sole
         `TITLE_MODEL` source of truth)

- [x] 4. @spec chat/oneshot-models Global per-harness oneshot preference: A configured oneshot model for a harness is stored globally

- [x] 5. @spec chat/oneshot-models Global per-harness oneshot preference: Preferences are keyed by harness not by project

- [x] 6. @spec chat/oneshot-models Oneshot model resolution: Configured model is used when it is in the catalog

- [x] 7. @spec chat/oneshot-models Oneshot model resolution: Missing or unknown config falls back to string-match default then first catalog model

- [x] 8. @spec chat/oneshot-models Oneshots use the resolved preference: Title and reply oneshots for a harness use that harness’s resolved oneshot model
