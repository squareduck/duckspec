# Archive list UX - Design

Duckboard-only: reverse-chronology archived lists, soft-archived explorations interleaved
with archived changes, and collapsed Archive sections by default.

## Approach

Stay in `crates/duckboard`. No duckpond / `ds archive` / `duckspec/archive/` layout
changes. Explorations remain in per-project `explorations.json` with a soft archive stamp;
chats stay under `chats/<id>/` until hard remove.

```
explorations.json          duckspec/archive/
┌──────────────────┐       ┌─────────────────────────┐
│ live (no stamp)  │──UI──▶│ (filesystem, ds archive) │
│ archived_at set  │       └───────────┬─────────────┘
└────────┬─────────┘                   │
         │                             │
         ▼                             ▼
    Archived list (Change + Dashboard)
    sort key desc: archived_at | folder prefix
```

```
Hover control (Change list, single leading control)
  live exploration     → ArchiveExploration  (one click, soft)
  archived exploration → RemoveExploration   (arm when sessions > 0, same as today)
```

## Archive sort (changes)

`build_changes` still loads via `read_sorted_dir` (ascending name). After loading
`archive/`, reverse the `Vec` so `YYYY-MM-DD-NN-*` is newest-first. Active `changes/`
order stays ascending.

```rust
// data.rs — only the archive load path
let mut archived_changes = build_changes(&root.join("archive"), "archive");
archived_changes.reverse();
```

Prefix format already guarantees reverse-lexical ≈ reverse-chronological (date, then
counter).

## Exploration model

Extend `chat_store::Exploration` with optional archive time. Absent / null = live. Serde
`default` keeps old JSON valid.

```rust
// chat_store.rs
pub struct Exploration {
    pub id: String,
    pub display_name: String,
    #[serde(default, alias = "card_id")]
    pub idea_path: Option<String>,
    /// Set when soft-archived in duckboard. ISO-8601 local string
    /// (same family as idea `created`). `None` = live.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archived_at: Option<String>,
    #[serde(skip)]
    pub session_count: usize,
}

impl Exploration {
    pub fn is_archived(&self) -> bool {
        self.archived_at.is_some()
    }
}
```

Stamp with the same local clock helper family used for idea timestamps (`iso8601_local` /
`current_local_datetime` pattern in `idea_store` / `chat_store`).

**Live lists** (Change picker, Dashboard Explorations):

```rust
e.idea_path.is_none() && !e.is_archived()
```

Idea-owned explorations stay Ideas-only (unchanged).

## Hover action (Change list)

Reuse the existing hover leading-control pattern in `area/change.rs`
(`hovered_exploration` + close button). One control, two meanings:

```
| State | Click | Arming |
| --- | --- | --- |
| Live | `ArchiveExploration(id)` — set `archived_at`, save, leave chats | Never (soft) |
| Archived | `RemoveExploration(id)` — drop from list, `delete_scope`, save | Same as today: skip arm when `session_count == 0`; else arm then commit |
```

Messages:

```rust
// area/change.rs
ArchiveExploration(String),
// existing:
ArmRemoveExploration(String),
RemoveExploration(String),
```

`ArchiveExploration` clears arm/hover as needed; does not delete chats. If the row was
selected, selection may stay so the user can still open the archived exploration scope.

Archived exploration rows in the Archived section also use hover → remove (same arm
rules). They need hover tracking on those rows the same way live rows do.

## Unified Archived list

Build a reverse-chronology row list for Change list + Dashboard Archived sections.

```rust
enum ArchivedEntry<'a> {
    Change(&'a ChangeData),
    Exploration(&'a Exploration),
}

fn sort_key_change(name: &str) -> String {
    // folder prefix "YYYY-MM-DD-NN" (or full name); empty if unparseable → sort last
}

fn sort_key_exploration(archived_at: &str) -> String {
    // ISO string compares lexicographically for same timezone shape;
    // use date-time prefix for interleave with YYYY-MM-DD
}

fn archived_entries<'a>(
    changes: &'a [ChangeData],
    explorations: &'a [Exploration],
) -> Vec<ArchivedEntry<'a>> {
    // non-idea-owned, is_archived explorations + all archived_changes
    // sort by key descending
}
```

Place the helper near list construction (e.g. `area/change.rs` or a small shared fn both
dashboard and change call) so both UIs share order and membership.

**Change list Archived section:**

- Empty only when both change archives and archived explorations are empty
- Change rows: existing branch icon + `SelectChange(full archive name)`
- Exploration rows: explore icon + `SelectChange(exp.id)` (scope still `Exploration`)
- Hover remove only on exploration rows

**Dashboard Archived:** same interleave and newest-first; click → existing navigation
messages (`ArchivedChangeClicked` / exploration click path). No hover remove on dashboard
(navigation-only today).

## Ideas Archive default

In `area/ideas.rs`, default expand is currently `unwrap_or(true)`. Archive starts
collapsed:

```rust
let expanded = state
    .section_expanded
    .get(&section)
    .copied()
    .unwrap_or(!matches!(section, IdeaState::Archive));
```

Inbox / Exploration / Change stay default-open. Ideas already sort by `created` desc — no
sort change.

Change area `"archived"` is already absent from default `expanded_sections` — keep that;
only ensure the section still auto-expands when selecting an archived change (existing
reveal path).

## Impact

- `explorations.json` gains optional `archived_at`; old files load as all-live

- No duckpond / CLI / cap tree impact unless specs later claim duckboard behavior under
  existing caps

- Unit tests: archive reverse order; exploration archive filter; sort interleave; ideas
  default expand

## Decisions

- **Soft flag vs move list** — `archived_at` on the same `Exploration` vec. Alternatives:
  parallel `archived_explorations` array (rejected: dual lists, more save paths).

- **No unarchive in this change** — proposal is archive + remove ladder only.
  Alternatives: unarchive control (rejected as scope creep).

- **Idea-owned explorations stay off Changes Archived** — Ideas owns that surface.
  Alternatives: also list them in Changes Archived (rejected: duplicate rows / wrong
  home).

- **Archive = one click; remove keeps arm** — soft vs destructive asymmetry matches
  existing empty-vs-armed remove.

- **Dashboard gets same interleave** — proposal “matching” Archived list; keeps hub
  consistent with Change column.

## Risks

- **ISO `archived_at` vs `YYYY-MM-DD-NN` interleave** → normalize both to a comparable
  date-time string (date + optional time/counter) so mixed rows sort predictably.

- **Stale armed remove after archive** → clear `armed_remove_exploration` on archive and
  on unhover (existing unhover already disarms).

- **Selected archived exploration still “open”** → allowed; scope and chats remain until
  remove.
