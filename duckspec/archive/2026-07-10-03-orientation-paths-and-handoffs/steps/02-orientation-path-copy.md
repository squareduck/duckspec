# Orientation path copy

Make every orientation path project-root-relative under `duckspec/`: change artifacts,
caps, codex, `project.md`, and the current-review pointer.

## Tasks

- [x] 1. In `crates/duckboard/src/scope.rs` `render_change_orientation`, use
         `duckspec/changes/{name}/` (including the facts-less fallback) and qualify the
         current review as `duckspec/changes/{name}/reviews/{r}`.

- [x] 2. Update caps and codex arms of `CurrentScopeHook` to point at `duckspec/caps/` +
         `duckspec/project.md` and `duckspec/codex/` + `duckspec/project.md` respectively.

- [x] 3. Extend unit tests so change orientation asserts the `duckspec/changes/{name}/`
         path; update caps orientation assertions for the new pointers; add a codex
         orientation test for the new scenario.

- [x] 4. Update `orientation_reports_highest_numbered_review` (and any related assertions)
         to expect the fully qualified review path.

- [x] 5. @spec session/scope Change identification and authority: Orientation names the scoped change as the default command target

- [x] 6. @spec session/scope Non-change scope orientation: A capability-tree scope carries no change facts

- [x] 7. @spec session/scope Non-change scope orientation: A codex scope points at the codex tree

- [x] 8. @spec session/scope Current review in orientation: Orientation reports the highest-numbered review as the current review
