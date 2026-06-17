# Archive backlink guard wiring

Wire the orphan-detection primitive into `ds archive`: run it before any write, refuse by
default, and add the `--allow-orphans` override.

## Prerequisites

- [ ] @step orphan-detection-primitive

## Tasks

- [x] 1. In `duckspec::cmd::archive`, after `execute_plan`, build the `projected` map from
         the `ArchiveResult` spec writes (cap path → `ProjectedSpec::Updated`) and call
         `audit::would_be_orphaned`; do this before `apply_results`/`rename` so a refusal
         leaves the tree untouched

- [x] 2. Refuse on detected orphans — abort with a message naming the offending source
         files — and add an `--allow-orphans` flag to `ds archive` that downgrades the
         refusal to a printed warning and proceeds

- [x] 3. @spec archive/backlink-guard Refusal and override: Refusal leaves the capabilities and change untouched

- [x] 4. @spec archive/backlink-guard Refusal and override: allow-orphans completes the archive with a warning
