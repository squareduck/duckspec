# Orphan detection primitive

Add the `duckpond` function that, given the capability specs a change would write, reports
which source backlinks the archive would orphan — reusing the audit's existing index and
scan.

## Prerequisites

- [ ] @step scan-boundary-exclusions

## Tasks

- [x] 1. In `duckpond::audit`, add a `ProjectedSpec` enum (`Updated(String)` / `Deleted`)
         and a
         `would_be_orphaned(project_root, duckspec_root, config,
         projected: &HashMap<String, ProjectedSpec>)`
         function that builds the current scenario index via `build_scenario_index`,
         applies the projection (drop each projected cap's keys, re-add keys parsed from
         its new content, or drop on `Deleted`), scans backlinks via `scan_source_files`,
         and returns the backlinks unresolved after but resolved before
         (`UnresolvedBacklink` set difference)

- [x] 2. @spec archive/backlink-guard Orphan detection: Archiving a change that removes a backlinked scenario flags the backlink

- [x] 3. @spec archive/backlink-guard Orphan detection: An archive that preserves every backlinked scenario reports no orphans

- [x] 4. @spec archive/backlink-guard Orphan detection: A backlink already unresolved before the archive is not attributed to it
