# Stream-tick CPU - Design

Split the stream UI timer from “turn open,” and precompute the Changed Files flat list so
iced `view()` only maps rows.

## Approach

```
today
  is_streaming (incl. awaiting) ──► StreamTick 10Hz + FlushTick 1Hz
                                          │
                                          ▼
                               full iced view() every fire
                                          │
                                          ▼
                         rebuild FileTree + flatten every view()

target
  needs_stream_tick? ──► StreamTick 10Hz   (animate +/or materialize)
  needs_flush_tick?  ──► FlushTick 1Hz     (dirty sessions only)
  set_changed_files / ToggleFileDir ──► rebuild cached flat rows
  view() ──► map cache → ListRow (no tree build)
```

Two independent fixes; either alone helps, together they match the proposal.

## Stream tick subscription

**Where:** `crates/duckboard/src/main.rs` (`subscription`, helpers next to
`any_session_streaming`); pure predicate in `area/interaction.rs` (or a tiny free fn next
to `should_materialize_on_stream_tick`) so it is unit-testable without `State`.

```rust
/// True when this session needs the 10 Hz stream UI tick.
/// Active agent work (streaming, not awaiting user) → animate dots.
/// Dirty + stick-to-bottom → materialize on tick (incl. rare awaiting edge).
fn session_needs_stream_tick(
    is_streaming: bool,
    is_awaiting_user: bool,
    chat_ui_dirty: bool,
    stick_to_bottom: bool,
) -> bool {
    if !is_streaming {
        return false;
    }
    if !is_awaiting_user {
        return true; // dots + possible materialize
    }
    // Mid-turn chips: agent quiet — tick only if deferred paint is owed.
    should_materialize_on_stream_tick(is_streaming, chat_ui_dirty, stick_to_bottom)
}

fn any_session_needs_stream_tick(state: &State) -> bool { /* fold sessions */ }
```

Subscription:

```rust
if any_session_needs_stream_tick(state) {
    // StreamTick @ TICK_MS (100)
}
if any_session_needs_flush_tick(state) {
    // FlushTick @ 1s — any ax.needs_flush (not merely is_streaming)
}
```

**Awaiting UX:** with no tick, pulsing dots freeze (or stay at last phase). Acceptable:
chips own the “your move” signal; dots mean “agent working.” Optional polish (not
required): hide `streaming_indicator` when `is_awaiting_user` — same files, one-line view
gate; can land with A if trivial.

**StreamTick handler:** unchanged logic (`bump_tick` + `should_materialize_on_stream_tick`
drain). Only who *subscribes* changes.

**FlushTick:** decouple from `is_streaming`. Fire when any session has `needs_flush` so
mid-turn persist still bounds loss without a 1 Hz wake after flush clears the flag.
`flush_dirty_sessions` already no-ops clean sessions.

## Changed Files row cache

**Where:** `crates/duckboard/src/area/change.rs` on `change::State`.

```rust
/// Owned flat rows for the Changed Files section.
/// Rebuilt when `changed_files` or `expanded_file_dirs` change — not in view().
changed_file_rows: Vec<ChangedFileRow>,

enum ChangedFileRow {
    Dir {
        key: String,
        name: String,
        depth: usize,
        is_expanded: bool,
        agg: Option<FileStatus>,
    },
    File {
        path: PathBuf,
        status: FileStatus,
        name: String,
        depth: usize,
    },
}

fn rebuild_changed_file_rows(
    files: &[ChangedFile],
    expanded: &HashSet<String>,
) -> Vec<ChangedFileRow> { /* today's FileTree insert + flatten, owned */ }
```

**Invalidate / rebuild:**

```
| Mutation | Rebuild |
| --- | --- |
| `State::set_changed_files` | yes (after expand auto-adjust) |
| `Message::ToggleFileDir` | yes |
| other expand edits that touch `expanded_file_dirs` | yes (only these two today) |
```

**View:** `view_changed_files_section` iterates `state.changed_file_rows` → `ListRow`
(status char, colors, `SelectChangedFile` / `ToggleFileDir`). Drop per-view
`FileTree::new` / `insert` / `flatten_file_tree` with borrowed rows — keep build helpers
for the rebuild path (or fold into `rebuild_changed_file_rows`).

`changed_files: Vec<ChangedFile>` stays the source of truth for dirty checks, explorer
tints, diffs, etc.

## Impact

- **`duckspec/caps/chat/stream-ui`** — doc (and any scenario that implies “tick while any
  session is streaming”) must say the stream UI tick is subscribed when a session needs
  animation or deferred pure-content materialize; idle mid-turn await does not keep the
  timer. Persist cadence: 1 Hz while sessions are dirty for flush, not while merely
  streaming.

- **duckboard only** — no duckpond / ds API change.

- **Tests** — pure `session_needs_stream_tick` cases; change-state tests that set/toggle
  rebuilds rows and `view` path does not re-tree (behavior via row cache content).

## Decisions

- **Predicate over “always tick while streaming”** — awaiting is `is_streaming` by design
  for open turns; gating on `!is_awaiting_user` (plus materialize edge) matches proposal
  without inventing a third turn phase.

- **Split StreamTick vs FlushTick gates** — animation/materialize at 10 Hz is the CPU bug;
  flush stays correctness-oriented and can be `needs_flush`-only so clean open turns do
  not wake the UI every second.

- **Owned row cache, not retained `FileTree`** — view only needs flat rows; expand toggles
  re-flatten from `changed_files` (cheap on user click, not on every frame).

- **No change to materialize rules** — `should_materialize_on_stream_tick` and structural
  immediacy stay as in stream-ui; only subscription frequency changes.

## Risks

- **Frozen dots while awaiting** → chips already communicate wait; optional hide indicator
  if it looks broken.

- **Missed materialize if dirty only while awaiting** → predicate still allows tick when
  `should_materialize_on_stream_tick` is true; UserChoiceRequest already
  force-materializes.

- **Stale cache if a new mutator forgets rebuild** → rebuild only through
  `set_changed_files` + `ToggleFileDir` (single chokepoints); document on the field.

## Open questions

None that block the approach. Optional hide-indicator-while-awaiting can be decided at
apply time (default: leave indicator code as-is, freeze is fine).
