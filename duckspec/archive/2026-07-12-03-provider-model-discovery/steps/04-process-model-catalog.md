# Process model catalog

Add a duckboard process-local model catalog refreshed once at app start, with
keep-last-good semantics, and wire pickers / context-window lookup to read only from the
catalog.

## Prerequisites

- [x] @step claude-host-model-discovery
- [x] @step grok-model-display-names

## Tasks

- [x] 1. Introduce a process `ModelCatalog` (per-harness slices, shared provider
         instances) in duckboard (e.g. `crates/duckboard/src/agent.rs` or a dedicated
         module)

- [x] 2. Implement refresh: each available provider rediscovers; success+non-empty
         replaces that harness slice; empty/failure keeps prior non-empty slice or leaves
         empty without panic

- [x] 3. Kick refresh once at app start (background / fire-and-forget OK)

- [x] 4. Point `available_models` and `model_context_window` at the catalog

- [x] 5. @spec harness/model-catalog Startup catalog refresh: App start refreshes models for each available provider

- [x] 6. @spec harness/model-catalog Startup catalog refresh: Successful refresh replaces that harness’s catalog slice

- [x] 7. @spec harness/model-catalog Keep last good on empty rediscovery: Empty rediscovery leaves the prior harness list intact

- [x] 8. @spec harness/model-catalog Keep last good on empty rediscovery: Cold failure leaves that harness empty without panic

- [x] 9. @spec harness/model-catalog Catalog is the selection source: Offered selectable models are the catalog contents

- [x] 10. @spec harness/model-catalog Catalog is the selection source: Context window lookup uses the catalog entry for the selected model
