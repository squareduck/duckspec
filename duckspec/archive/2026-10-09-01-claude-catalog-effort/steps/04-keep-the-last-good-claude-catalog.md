# Keep the last good Claude catalog

A failed Claude refresh keeps the last good slice. Grok and Codex still clear.

## Prerequisites

- [x] @step advertise-the-claude-code-catalog

## Context

Parsed catalog status lives only on `InitResult.catalog`
(`CatalogStatus::Ok { expires_at: Option<String> }` or `Failed`). A missing catalog object
is ok with no expiry. `expires_at` is the RFC3339 string from `expiresAt`, or absent.
`list_models` and the hot runtime keep models and drop the catalog status, so retention
has to read the handshake result. It is not on `ModelInfo`.

The agent now always emits `modelState.catalog`. A failed fetch is `status: failed` with
an empty `availableModels`. Claude `to_model_info` copies `effort` onto `ModelInfo`.
`discover_models` still caches the first handshake for the process and turns a spawn or
protocol failure into an empty vec. It does not apply the last-good ladder.

## Tasks

- [x] 1. Keep a Claude provider memo and the snapshot at
         `~/.config/duckboard/claude-catalog.json` in `crates/duckboard/src/agent.rs`.
         Load rows as stored under `claude-code`.

- [x] 2. Apply the failure ladder and the success replace, including an applied empty
         slice staying empty.

- [x] 3. After app start, wake Claude at expiry and retry every five minutes while its
         slice is empty or past expiry. Extend `model_catalog_ready_subscription` in
         `crates/duckboard/src/main.rs`.

- [x] 4. Retarget the existing clear-slice tests to a harness that does not keep a
         last-good catalog.

- [x] 5. @spec harness/model-catalog Claude catalog retention: Failed Claude refresh keeps the last good catalog

- [x] 6. @spec harness/model-catalog Claude catalog retention: Successful Claude refresh replaces the catalog

- [x] 7. @spec harness/model-catalog Claude refresh schedule: Claude is refreshed again only when its catalog is due

## Outcomes

`list_models` still caches the first handshake and drops catalog status. Retention calls
`ClaudeCodeProvider::force_refresh`. The provider memo and `expires_at` are fields on
`ModelCatalog`, not on the provider: apply is what replaces the memo, and a snapshot
install does not.
