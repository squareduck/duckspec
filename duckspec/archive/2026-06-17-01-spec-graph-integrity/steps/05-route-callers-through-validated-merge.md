# Route callers through validated merge

Replace the open-coded apply-then-parse in the three call sites with the validated
wrappers, surfacing failures instead of swallowing them.

## Prerequisites

- [ ] @step validated-merge-wrappers

## Context

These call sites are the surfacing behavior the change promises but the specs don't
contract directly. Each routes through the wrappers from step 04 and changes how it
handles failure:

- `archive::execute_plan` already aborts on a merge failure — switch it to
  `merge_spec_delta` / `merge_doc_delta` chosen by the target filename (`spec.md` vs
  `doc.md`) and abort on either error variant.

- `status::delta_new_coverage` currently swallows every failure and returns empty coverage
  — switch to `merge_spec_delta`, reuse `Merged::Updated.artifact` instead of re-parsing,
  and on error emit one visible error line and keep going (the dashboard must not abort).

- `audit::build_change_scenarios` currently swallows every failure — switch to
  `merge_spec_delta`, reuse the parsed artifact, and on error record a counted
  `ChangeMergeError` entry. Add a `change_merge_errors: Vec<ChangeMergeError>` field to
  `AuditReport` and fold it into `total_errors()`.

## Tasks

- [x] 1. Route `archive::execute_plan` through `merge_spec_delta` / `merge_doc_delta`
         selected by target filename, aborting on either error

- [x] 2. Route `status::delta_new_coverage` through `merge_spec_delta`, reusing the parsed
         spec, printing a visible error line on failure and continuing

- [x] 3. Add `ChangeMergeError` and the `change_merge_errors` report field, and route
         `audit::build_change_scenarios` through `merge_spec_delta`, recording a counted
         entry on failure
