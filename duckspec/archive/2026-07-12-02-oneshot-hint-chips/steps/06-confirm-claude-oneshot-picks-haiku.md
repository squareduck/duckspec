# Confirm Claude oneshot picks haiku

Implement and cover Claude oneshot preferred-model selection: prefer curated `haiku` when
advertised; otherwise fall back to another advertised model.

## Prerequisites

- [x] @step raise-oneshot-call-budget

## Context

Delta: `caps/harness/claude` — requirement **Oneshot preferred model**. Code lives around
`TITLE_MODEL` / oneshot runtime preferred model and `AcpOneshotRuntime::pick_model`.

## Tasks

- [x] 1. Ensure Claude oneshot path prefers the curated `haiku` alias when that model is
         among advertised models (fix selection if already correct)

- [x] 2. @spec harness/claude Oneshot preferred model: Preferred oneshot model is selected when advertised

- [x] 3. @spec harness/claude Oneshot preferred model: Oneshot model falls back when preferred is absent

- [x] 4. Optional: `tracing::debug` (or equivalent) when oneshot selects a model id for
         live confirmation

- [x] 5. Run `cargo test -p duckchat` for the new selection coverage
