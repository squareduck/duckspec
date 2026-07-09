# Harness identity types

Give duckchat a harness dimension: models carry the harness that owns them and their
context window, and a `ModelRef` becomes the persisted unit of model choice with a legacy
shim.

## Tasks

- [x] 1. Extend `ModelInfo` in `crates/duckchat/src/provider.rs` with a `harness: String`
         field and a `context_window: Option<usize>` field.

- [x] 2. Add a `ModelRef { harness: String, model: String }` type (in `provider.rs` or a
         small shared module), `Serialize`/`Deserialize`, re-exported from `lib.rs`.

- [x] 3. Implement `ModelRef::parse_legacy(&str)` mapping a bare model-id string to the
         `claude-code` harness, and a deserialize path that accepts either the struct form
         or a legacy bare string.

- [x] 4. Update `ClaudeCodeProvider::list_models` in `crates/duckchat/src/claude_code.rs`
         to tag every returned `ModelInfo` with the `claude-code` harness.

- [x] 5. @spec harness/selection Harness-tagged model identity: A model choice round-trips its harness and model

- [x] 6. @spec harness/selection Harness-tagged model identity: A legacy bare model id loads as the Claude harness
