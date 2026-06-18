# Reconcile returns relocations

Give `idea_store::reconcile` a return value describing the file moves it performed, and
cover the drift classification and relocation reporting with tests.

## Context

`drift_target` is private, so the classification scenarios are unit-tested inline in
`idea_store.rs` under the existing `#[cfg(test)] mod tests`, calling `drift_target`
directly with no I/O.

`reconcile` performs real file I/O via `save_idea` into `ideas_root(project_root)`. That
path resolves under `config::config_dir()` (hardcoded `~/.config/duckboard`);
`project_root` is only a hash seed, so pointing it at a temp dir does **not** isolate the
writes. Task 1 therefore adds a `#[cfg(test)]` thread-local override to `config_dir` that
the tests set to a temp dir via the test module's existing `tempdir()` helper. The
reconcile tests then seed an `Idea` on disk in the `change/` subtree (via `save_idea`),
populate `ProjectData.active_changes` / `archived_changes` to drive each outcome, and
assert the returned moves plus the resulting on-disk location and frontmatter.
`data::strip_archive_prefix` maps an archived change directory name
(`YYYY-MM-DD-NN-<base>`) back to its base change name.

## Tasks

- [x] 1. Add `IdeaMove { old_path, new_path, title }` and change `reconcile` to return
         `Vec<IdeaMove>`
  - [x] 1.1 Define the `IdeaMove` struct in `idea_store.rs`
  - [x] 1.2 In `reconcile`, capture `prev_path` before `save_idea` and push an `IdeaMove`
            when `idea.abs_path` changed; leave `drift_target` unchanged
  - [x] 1.3 Update the startup call site at `main.rs:175` to discard the returned moves
  - [x] 1.4 Add a `#[cfg(test)]` thread-local override to `config::config_dir` and a
            setter for tests; reuse the existing `tempdir()` test helper (no new
            dependency needed)

- [x] 2. @spec ideas/reconcile Change-linked drift classification: Linked change archived classifies the idea as via-change

- [x] 3. @spec ideas/reconcile Change-linked drift classification: Linked change gone classifies the idea as orphaned

- [x] 4. @spec ideas/reconcile Change-linked drift classification: Active linked change leaves the idea unchanged

- [x] 5. @spec ideas/reconcile Change-linked drift classification: Already-archived idea keeps its archive reason

- [x] 6. @spec ideas/reconcile Relocation reporting: An archiving relocation is reported with source and destination

- [x] 7. @spec ideas/reconcile Relocation reporting: A no-op reconciliation reports no relocations
