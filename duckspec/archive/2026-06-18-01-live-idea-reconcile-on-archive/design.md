# Live idea reconciliation on change archival — Design

Give `idea_store::reconcile` a return value describing the file moves it performed, then
call it from `reload_and_reconcile` and drive `area::ideas::refresh_after_move` for each
move so a live archival follows the open tab and list selection.

## Approach

The drift logic is already correct; the only structural change is that `reconcile`
*reports* its moves instead of swallowing them, and the live archival path *calls* it.

```text
ds archive (external, while running)
        │  file watcher → tree_changed
        ▼
reload_and_reconcile(state)                         [main.rs]
  state.project.reload()                            ← fresh archived_changes
  …promotion / subscription migration (unchanged)…
  moves = idea_store::reconcile(&mut state.ideas.ideas, &state.project)
        │                                            ┌─ drift_target (unchanged)
        │  for each idea:  drift? → save_idea ───────┤   ViaChange / Orphaned
        │                  capture (old→new, title)  └─ moves file change/→archive/
        ▼
  for mv in moves:
      area::ideas::refresh_after_move(&mut state.ideas, &mut state.tabs,
                                      &mv.old_path, &mv.new_path, &mv.title)
        └─ re-points state.selected and the pinned `idea:` tab id + title
```

`drift_target` is untouched — it already returns `ViaChange` when the linked change is in
`archived_changes` and `Orphaned` when the change directory has vanished. The only
behavioral additions are the move report and the live call site.

## `IdeaMove` and the `reconcile` return value

`reconcile` already mutates each drifted idea in place and calls `save_idea`, which
renames the file and updates `idea.abs_path` (`idea_store.rs:449`). We capture the
before/after path around that call and collect them. The startup caller ignores the
result; the live caller uses it.

```rust
/// A reconcile-driven relocation of an idea's file. `title` is the
/// post-move display title, used to refresh the pinned tab label.
pub struct IdeaMove {
    pub old_path: PathBuf,
    pub new_path: PathBuf,
    pub title: String,
}

pub fn reconcile(ideas: &mut [Idea], project: &ProjectData) -> Vec<IdeaMove> {
    let project_root = project.project_root.as_deref();
    let mut moves = Vec::new();
    for idea in ideas.iter_mut() {
        let Some((new_state, archived)) = drift_target(idea, project) else {
            continue;
        };
        idea.state = new_state;
        idea.frontmatter.archived = archived;
        let prev_path = idea.abs_path.clone();
        let body = read_body(&idea.abs_path).unwrap_or_default();
        if let Err(e) = save_idea(idea, &body, project_root) {
            tracing::warn!("failed to reconcile idea: {e}");
            continue;
        }
        if idea.abs_path != prev_path {
            moves.push(IdeaMove {
                old_path: prev_path,
                new_path: idea.abs_path.clone(),
                title: idea.display_title(),
            });
        }
    }
    moves
}

// drift_target — unchanged.
```

Returning an owned `Vec<IdeaMove>` ends the `&mut state.ideas.ideas` borrow before the
caller needs `&mut state.ideas` again for `refresh_after_move`, so there is no borrow
conflict to fight.

## Live wiring in `reload_and_reconcile`

The reconcile step slots in after `state.project.reload()` and the existing
externally-archived-change detection loop, before `refresh_obvious_command`. It runs on
both triggers that share this function: manual `Refresh` (`main.rs:438`) and the
file-watch `tree_changed` path (`main.rs:863`).

```rust
fn reload_and_reconcile(state: &mut State) -> bool {
    // …existing: snapshot names, state.project.reload(), promotion,
    //   subscription migration for new_archived…

    let moves = idea_store::reconcile(&mut state.ideas.ideas, &state.project);
    for mv in moves {
        area::ideas::refresh_after_move(
            &mut state.ideas,
            &mut state.tabs,
            &mv.old_path,
            &mv.new_path,
            &mv.title,
        );
    }

    area::change::refresh_obvious_command(&mut state.interactions, &state.project);
    archived_any
}
```

The `bool` return is unchanged. It signals "tab ids were rewritten to new *archive change*
paths, so re-read those tab bodies from disk." Idea moves do **not** feed it: a pinned
idea tab's body is unchanged by a state move, and `refresh_open_tabs` reads tab bodies via
`project.read_artifact("idea:…")`, which does not serve idea tabs (ideas live in the data
dir, outside the project). `refresh_after_move` already re-points the tab id and title,
which is all an idea move needs.

## Startup call site

`main.rs:175` keeps calling `reconcile`, now discarding the moves — at project open the
tabs are freshly `default()` and nothing is selected, so there is no UI to follow.

```rust
let _ = idea_store::reconcile(&mut self.ideas.ideas, &self.project);
```

## Tests

`drift_target` is pure (`Idea` + `ProjectData` →
`Option<(IdeaState,
Option<ArchiveKind>)>`) and private, so it is unit-tested inline in
`idea_store.rs` under the existing `#[cfg(test)] mod tests`. `reconcile`'s move reporting
and on-disk relocation are exercised against a temp ideas root.

- `drift_target` — change in `archived_changes` → `(Archive, ViaChange)`; change absent
  from both active and archived → `(Archive, Orphaned)`; change still active → `None`;
  idea already in `Archive` → `None`; idea with no `change` field → `None`.

- `reconcile` — a change-state idea whose change is archived returns one `IdeaMove` from
  the `change/` subtree to the `archive/` subtree, the file exists at the new path and not
  the old, and the frontmatter records `ViaChange`.

## Decisions

- **`reconcile` returns `Vec<IdeaMove>`** — chosen so the caller has exactly the
  old→new→title triples it needs for `refresh_after_move`. Alternatives: caller diffs a
  path snapshot taken before/after (rejected: reconstructs information reconcile already
  holds); pass a UI callback into `reconcile` (rejected: couples the storage layer to
  duckboard UI types).

- **Reuse `area::ideas::refresh_after_move`** — it already re-points `state.selected` and
  the pinned tab for the move-driven flows (Explore, promotion, manual archive). Reused
  rather than writing a parallel follow path.

- **Idea moves excluded from the `bool` return** — the bool's contract is "re-read
  archive-change tab bodies"; idea bodies are unchanged by a state move and are not served
  by `refresh_open_tabs`, so folding them in would trigger pointless re-reads.

- **`drift_target` left untouched** — the `ViaChange` / `Orphaned` decision is already
  correct; the bug is purely the missing live call site and the swallowed moves.

- **Config-dir test seam for hermetic reconcile tests** — `reconcile` writes through
  `save_idea` -> `ideas_root` -> `config::data_dir` -> `config::config_dir`, which is
  hardcoded to `~/.config/duckboard` and ignores `project_root` (the project path is only
  a hash seed). Pointing `project_root` at a temp dir does not isolate the I/O, so a
  `#[cfg(test)]` thread-local override is added to `config_dir`, set per test to a temp
  dir via the test module's existing `tempdir()` helper. Thread-local (not an env var)
  keeps it parallel-safe without a serialization dependency, and `#[cfg(test)]` gating
  means no production behavior change. Alternatives: env-var override (rejected:
  process-global, needs serializing parallel tests); thread an ideas-root through
  `reconcile`/`save_idea` (rejected: ripples across `save_idea`'s callers).

## Risks

- **Borrow conflict between `reconcile` and `refresh_after_move`** → `reconcile` returns
  an owned `Vec`, releasing the `&mut state.ideas.ideas` borrow before the follow loop
  reborrows `&mut state.ideas`.

- **Multiple ideas linked to the same archived change** → each idea is evaluated
  independently in the `reconcile` loop and emits its own `IdeaMove`; the follow loop
  handles each.

- **Stale in-memory ideas list** → out of scope by design; the linked idea is already in
  the in-memory list (loaded at open or created in-session), so the live reconcile sees
  it. External idea edits are explicitly deferred.

## Open questions

None. The one prior question — whether an idea body can live outside the preview pinned
tab — is resolved: an idea body only ever occupies `tabs.preview`, which is the single
surface `refresh_after_move` already re-points. This is treated as an invariant; if a user
forces an idea body into a separate `file_tab`, that tab going stale is acceptable and out
of scope.
