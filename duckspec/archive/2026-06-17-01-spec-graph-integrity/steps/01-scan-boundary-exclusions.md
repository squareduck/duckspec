# Scan boundary exclusions

Close the `@spec` scan boundary in `duckpond`: add the `exclude` config key and prune
nested duckspec projects and excluded paths from the audit walk.

## Tasks

- [x] 1. In `duckpond::config`, add `exclude: Vec<PathBuf>` to `Config` and parse it in
         `Config::load` the same way as `test_paths` (array of strings, default empty),
         adding `ConfigError::BadExclude` for a non-array value

- [x] 2. In `duckpond::audit::scan_source_files`, canonicalize the `exclude` entries
         (relative to `project_root`) and add a `WalkBuilder::filter_entry` closure that
         prunes any directory owning its own `duckspec/caps/` and any path under an
         excluded entry; keep the existing duckspec-root skip

- [x] 3. @spec audit/scan-boundary Scan roots: Configured test_paths scope the scan

- [x] 4. @spec audit/scan-boundary Scan roots: Empty test_paths scans from the project root

- [x] 5. @spec audit/scan-boundary Excluded paths: Excluded file and excluded directory subtree contribute no backlinks

- [x] 6. @spec audit/scan-boundary Excluded paths: Non-array exclude raises BadExclude

- [x] 7. @spec audit/scan-boundary Nested duckspec projects: A nested project is skipped while the enclosing project is still scanned
