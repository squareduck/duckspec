# Clear clippy hygiene

Clear post-change clippy noise: unused import and dead oneshot under-input helper in
non-test builds. No new capability scenarios.

## Prerequisites

- [x] @step confirm-claude-oneshot-picks-haiku

## Tasks

- [x] 1. Drop unused `CONTENT_PAD_Y` import from
         `crates/duckboard/src/widget/text_edit.rs` (or use it if still required)

- [x] 2. Mark `oneshot_under_input_chrome_visible` as `#[cfg(test)]` or fold tests onto
         eligibility-only helpers and remove the public dead symbol

- [x] 3. Run `cargo clippy -p duckboard -p duckchat --all-targets` and
         `cargo test -p duckboard -p duckchat`; fix remaining warnings from this change
