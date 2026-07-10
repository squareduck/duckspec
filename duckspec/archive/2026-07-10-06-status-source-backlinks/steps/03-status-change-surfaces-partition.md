# Status change surfaces partition

Wire `ds status <change>` to `for_change` so the test-coverage section reports linked vs
open from source backlinks, never from marker path lists, and never as a false missing
list for linked work.

## Prerequisites

- [x] @step change-coverage-helper
- [x] @step library-coverage-tests

## Tasks

- [x] 1. In `status_change`, load `Config`, resolve `project_root`, call
         `change_coverage::for_change`, and replace the marker-path coverage tally for
         change status

- [x] 2. Render linked fraction and list only open scenarios as open progress; never list
         a linked scenario as missing or open; surface merge errors as visible lines and
         continue

- [x] 3. Leave global `ds status`, single-file `status_spec` / `status_spec_delta`, steps,
         and reviews chrome unchanged

- [x] 4. @spec status/change-coverage Change status surfaces the partition: Open scenario appears in change status open list

- [x] 5. @spec status/change-coverage Change status surfaces the partition: Linked scenario does not appear as missing or open
