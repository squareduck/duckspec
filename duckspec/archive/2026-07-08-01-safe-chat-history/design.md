# Safe chat history — Design

Gate exploration-to-change promotion strictly on the authoritative `ds create change`
binding, and make the chat store lose-proof through atomic writes, non-destructive scope
migration, flush-before-mutate, and debounced eager persistence.

## Approach

Two independent hardening tracks that meet at promotion. Upstream fixes *whether* a
promotion fires (attribution); downstream fixes what happens to chat history when any
scope mutation runs (durability). Either track alone would have prevented the incident;
together they are defense-in-depth.

```
 file-watcher reload (main.rs:865)
        │
        ▼
 reload_and_reconcile
        │  detects a change dir not in the previous snapshot
        ▼
 ┌─────────────────────────────┐   binding?   ┌──────────────────────────┐
 │ pending_bindings.get(name)? │─── no ──────►│ do nothing (plain change)│
 └─────────────────────────────┘              └──────────────────────────┘
        │ yes (authoritative: an exploration ran `ds create change`)
        ▼
 route_promotion ──► promote_* ──► migrate scope A → B
        │                               │
        │                    ┌──────────┴───────────┐
        │                    ▼                      ▼
        │            in-memory MERGE          on-disk MERGE
        │        (interactions, dedup)     (per-file, non-clobber)
        ▼
 flush_before_mutate: persist every dirty session first
```

```
 streaming turn (AgentEvent loop, main.rs:1331)
   send ──► save ──► [msg, msg, msg, …] ──► turn-complete ──► save
                        │  each mutation marks the session dirty
                        ▼
              debounced flush (~1s, coalesced) + flush at turn
              boundary / before mutate / on quit
```

The load-bearing fix is **flush-before-mutate + merge** — it closes the exact incident.
Binding-gating removes the trigger that caused it; debounced eager saves defend the
remaining hard-crash window.

## Binding-gated promotion (`exploration/promotion`)

`reload_and_reconcile` currently detects a new change directory by diffing the pre-reload
in-memory snapshot against the reloaded project, then — when no `pending_bindings` entry
exists — falls back to `fallback_exploration_id`, which guesses the owner from whatever
the UI is focused on. That guess is the root cause: a VCS reappearance (jj checkout,
unarchive) reads as "new", and the focused-but-unrelated exploration gets adopted.

The only sound signal is the binding staged at the causal moment — an exploration
session's agent running `ds create change <slug>` (`main.rs:1379`, committed to
`pending_bindings` at `main.rs:1494`). Promotion keys off that and nothing else.
`fallback_exploration_id` is deleted.

```rust
// reload_and_reconcile — the attribution decision
if let Some(new_name) = first_change_not_in(&old_change_names) {
    // Authoritative only. No focus-based fallback: an unbound new dir is a
    // plain change (out-of-band create, VCS reappearance, unarchive) and is
    // left alone — never adopts an unrelated exploration's chat.
    if let Some(exp_id) = state.change.pending_bindings.remove(&new_name) {
        route_promotion(state, &exp_id, &new_name);
    }
}

// deleted:
// fn fallback_exploration_id(state: &State) -> Option<String> { … }
```

A VCS reappearance now costs nothing: detected as "new", no binding, no action. The
trade-off (a change created fully out-of-band while an exploration is focused will not
auto-migrate that exploration's chat) is rare and non-destructive — the chat stays under
its own scope.

## Atomic session writes (`chat/persistence`)

`save_session` uses `std::fs::write`, which truncates-then-writes: a crash or concurrent
reader mid-write can observe a truncated or empty file. Route every session write through
a temp-file + `rename` helper (atomic on the same filesystem).

```rust
// chat_store.rs
fn write_atomic(path: &Path, data: &[u8]) -> io::Result<()> {
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, data)?;
    std::fs::rename(&tmp, path)   // atomic replace
}

pub fn save_session(session: &ChatSession, project_root: Option<&Path>)
    -> anyhow::Result<()> { /* build PersistedSession, write_atomic(...) */ }
```

## Non-destructive scope migration (`chat/persistence`)

Two clobber sites turn a scope rename into data loss:

- **On disk:** `rename_scope` does `fs::rename(old_dir, new_dir)`, which fails outright
  when `new_dir` exists — leaving the source orphaned (the stray `chats/exploration-…/` in
  the incident).

- **In memory:** `promote_exploration` / `promote_idea_exploration` do
  `interactions.insert(Scope::Change(name), ix)`, which **replaces** any existing
  InteractionState for that change — discarding its live sessions.

Both become merges. On disk, move session files individually; on same-id collision keep
the fuller copy and set the loser aside rather than delete it. In memory, fold sessions
into the existing InteractionState (dedup by id, keep fuller), preserving the target's
`instance_id` so its PTY/agent subscriptions survive.

```rust
// chat_store.rs — merge, never clobber
pub fn merge_scope(from: &str, to: &str, project_root: Option<&Path>) {
    // for each <id>.json in chats/<from>/:
    //   if chats/<to>/<id>.json absent      -> move it
    //   else keep the one with more messages, rename the loser <id>.json.orphan
    todo!()
}

// interaction.rs — fold, don't overwrite
pub fn merge_sessions(into: &mut InteractionState, incoming: Vec<AgentSession>) {
    // append sessions whose id is absent in `into`; on id collision keep the
    // one with more messages; re-run reconcile_display_names. `into.instance_id`
    // (subscriptions) is untouched.
    todo!()
}
```

Promotion becomes: flush → if target scope exists, `merge_sessions` into it; else insert
as today → `merge_scope` on disk.

## Flush-before-mutate + eager persistence (`chat/persistence`)

The incident lost 296 messages because promotion mutated in-memory state while a turn's
streamed messages lived only in memory (persistence happens at send and turn-complete,
`interaction.rs:1370` / `main.rs:1414`). Two mechanisms close this:

**Flush-before-mutate.** Any code path that migrates, replaces, or drops an
InteractionState first persists every dirty session it holds. This is the guarantee that
makes the incident impossible regardless of attribution.

```rust
fn flush_sessions(ix: &InteractionState, project_root: Option<&Path>) {
    for ax in &ix.sessions {
        let _ = chat_store::save_session(&ax.session, project_root);
    }
}
// called at the top of promote_exploration / promote_idea_exploration,
// before any interactions.remove / insert / merge.
```

**Debounced eager save.** Each message-mutating `AgentEvent` (`ContentDelta` flush,
`ToolUse`, `ToolResult`) marks the active session dirty; a coalesced timer flushes at most
~1s while streaming. Full-file rewrite per message would be O(n²) over a turn (the
incident's largest session is ~1 MB / 732 messages), so the timer bounds both the write
cost (O(n) per interval) and the crash-loss window (~1s). Turn boundaries, pre-mutate, and
app quit force an immediate flush.

```rust
// interaction / AgentEvent handling
struct DirtyFlush { dirty: bool, last_flush: Instant }
// on ToolUse / ToolResult / text-flush: mark dirty
// on tick (~1s) or TurnComplete or before-mutate or quit: if dirty -> save_session
```

## Decisions

- **Promote only on `pending_bindings`; delete `fallback_exploration_id`** — the binding
  is the sole causal signal. Alternatives: keep the fallback but verify exploration↔change
  relatedness (rejected: no reliable relatedness signal exists — the misattribution
  *looked* related to focus); persist a known-change set to suppress VCS-reappearance
  false positives (unnecessary once the action is binding-gated — a false "new" detection
  with no binding is already a harmless no-op).

- **`pending_bindings` stays in-memory** — the create→detect window is one file-watcher
  tick; a loss there is non-destructive (change and exploration both survive
  independently). Alternative: persist to `bindings.json` (rejected: stale-entry lifecycle
  not worth the millisecond window).

- **Debounced single-JSON writes, ~1s coalesced** — bounds IO and loss window without a
  format change. Alternatives: per-message full rewrite (rejected: O(n²)); JSONL
  append-log / sidecar with compaction (rejected *for this change*: adds a second file,
  compaction, and crash-recovery-on-load — the format change the proposal scoped out;
  revisit as a future change if hard-crash durability becomes a priority).

- **Merge collision keeps the fuller copy, never deletes the loser** — the loser is set
  aside as `<id>.json.orphan`. Alternative: keep both as distinct sessions (rejected:
  duplicate ids confuse the session bar and dedup logic).

## Risks

- **Clean quit/suspend flush depends on an app shutdown hook** → verify duckboard's iced
  loop exposes a shutdown / `window::close` path to force a final flush; if none exists,
  the debounce interval bounds the loss and turn-boundary saves still cover the common
  case.

- **`rename`-based atomic write assumes same filesystem** → the temp file lives in the
  destination directory, so `rename` stays intra-filesystem; document the assumption.

- **Merge dedup picks the wrong copy on a genuine id collision** → resolution is
  non-destructive (loser preserved as `.orphan`), so a wrong pick is recoverable, never
  lost.

## Open questions

- None — the three prior open questions (persist bindings, debounce trigger, append-log vs
  single-JSON) are resolved in Decisions.
