# Change coverage helper

Extract shared change-scenario projection and source scanning from audit, and add
`duckpond::change_coverage::for_change` that partitions change-introduced `test:code`
scenarios into linked vs open by resolving source `@spec` keys.

## Tasks

- [x] 1. Add `crates/duckpond/src/change_coverage.rs` with `ChangeCoverage`,
         `ChangeCoverageError`, and `for_change(...)`; export the module from `lib.rs`

- [x] 2. Make scenario projection and source scan reusable from both `run_audit` and
         `for_change` (lift private helpers as needed; do not publicize a large audit
         grab-bag)

- [x] 3. Implement `for_change`: project change-introduced scenarios, keep `test:code`
         only, scan source backlinks with the same `Config` boundary as audit, partition
         into `linked` / `open`; record merge errors and continue

- [x] 4. Ensure marker path lists are never consulted for linkage; step checkbox state is
         not an input

- [x] 5. Keep existing audit behavior green (`cargo test -p duckpond`)
