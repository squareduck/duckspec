# Render pending vs error in ds audit

Surface the new pending bucket in the `ds audit` CLI output, distinct from errors, without
changing the exit code.

## Prerequisites

- [ ] @step classify-unlinked-scenarios-in-the-change-audit

## Tasks

- [x] 1. In `crates/duckspec/src/cmd/audit.rs` `print_report`, reword the
         `missing_backlink_scenarios` line to read as a checked-off-but-unlinked defect.

- [x] 2. Add a section rendering `pending_backlink_scenarios` as dimmed informational
         lines (`·`), clearly marked "pending / not yet implemented".

- [x] 3. When the audit has no errors but does have pending scenarios, extend the success
         line to note the pending count, so a clean-but-incomplete change reads as
         in-progress rather than done.
