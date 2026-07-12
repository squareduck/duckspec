# Catalog clear on empty rediscovery

Flip process model catalog empty-rediscovery policy so a harness slice is cleared instead
of kept last-good.

## Tasks

- [x] 1. Change `ModelCatalog::apply_harness` in `crates/duckboard/src/agent.rs` to always
         write the discovered slice (empty clears prior models)

- [x] 2. @spec harness/model-catalog Clear slice on empty rediscovery: Empty rediscovery clears the prior harness list

- [x] 3. @spec harness/model-catalog Clear slice on empty rediscovery: Cold failure leaves that harness empty without panic
