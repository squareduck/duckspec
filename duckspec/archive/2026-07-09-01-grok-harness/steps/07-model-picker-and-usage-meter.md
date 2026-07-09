# Model picker and usage meter

Group the model picker's choices by harness and drive the usage meter's denominator from
the selected model's context window.

## Prerequisites

- [ ] @step harness-dispatch

## Context

The picker choices are built in `crates/duckboard/src/widget/agent_chat.rs`
(`model_entries` / `chat_model_choices`), rendered from `available_models()`. Group
entries under their owning harness. The usage meter today takes its context window from
the `UsageUpdate` event; make the denominator the selected model's `context_window` (from
`ModelInfo`), and when it is unknown show no fill rather than computing against an assumed
window.

Note from step 05: `ModelChoice.id` is still a bare model-id `Option<String>`, and the
harness dimension is bridged at the two selection sites — `ModelSelected` in
`interaction.rs` and `ModelDefaultSelected` in `settings.rs` both wrap a picked id as
`ModelRef::new("claude-code", id)`. That hardcoded `"claude-code"` is a transitional shim
that only holds while the picker offers Claude-only models; when task 1 makes the choices
harness-aware, carry the real harness on `ModelChoice` and drop those two hardcoded tags
so a picked grok model persists as the grok harness. The view sites read
`selected_model`/`project_model_default` back through `.map(|r| r.model.as_str())`, which
also collapses the harness.

## Tasks

- [x] 1. Build the picker choices grouped/labeled by each model's harness in
         `agent_chat.rs`.

- [x] 2. Compute the usage meter's fill from the selected model's `context_window`,
         showing no fill when the window is unknown.

- [x] 3. @spec harness/model-picker Harness-grouped choices: Choices present each model under its harness

- [x] 4. @spec harness/model-picker Context fill from the active model's window: Fill is measured against the selected model's window

- [x] 5. @spec harness/model-picker Context fill from the active model's window: A model with no known window shows no fill
