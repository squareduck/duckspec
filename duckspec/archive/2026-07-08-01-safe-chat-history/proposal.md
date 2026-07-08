# Safe chat history

Make duckboard's chat history durable against in-flight loss, and stop
exploration-to-change promotion from misattributing an unrelated exploration to a change.

## Motivation

A real incident lost chat history. The `comptime` change reappeared on disk after its own
`/ds-apply` agent ran `jj abandon` / `jj edit`, churning the working copy. duckboard's
reload read that reappearance as a *new* change and, finding no authoritative binding,
promoted whatever exploration the UI happened to be focused on — an unrelated "interactive
book" idea — into `comptime`. Two failures compounded:

- **Misattribution.** The idea's chat was commingled into the `comptime` scope and its
  frontmatter was stamped `change: comptime`.

- **Data loss.** Promotion overwrote the live in-memory `comptime` interaction state,
  discarding an in-flight `/ds-apply` session (296 streamed messages) that had only been
  persisted as a prompt stub. It was recoverable *only* because Claude's own transcript
  survived — duckboard's store did not.

Why now: the history was recovered by hand this time, but the underlying fragility
remains. No single logic bug should ever be able to silently destroy chat history, and
promotion should never bind a change to an exploration it did not actually originate from.

## Scope

```
caps/
├── chat/                       ← NEW namespace
│   └── persistence/            ← NEW
├── exploration/                ← NEW namespace
│   └── promotion/              ← NEW
├── ideas/reconcile/            (unchanged)
└── session/scope/              (unchanged)
```

### New capabilities

- `chat/persistence` — the durability guarantee for chat sessions: atomic writes (temp
  file + rename), eager per-message persistence to shrink the send-to-turn-complete loss
  window, flush-before-mutate, and scope migration that merges into an existing scope and
  never clobbers or drops a same-id session file.

- `exploration/promotion` — the attribution policy when a new change directory is
  detected: promote only on an authoritative session binding, treat a VCS-driven
  reappearance or unarchive as not-new, and never infer a change's originating exploration
  from current UI focus.

### Out of scope

- The chat rendering UI (message widgets, session bar).

- Importing Claude's transcript as a product feature — it was used only for the one-off
  manual recovery.

- `ideas/reconcile` behavior — it follows the change link that promotion writes; it is not
  modified here.

- Any on-disk chat/session format change — persistence must stay backward-compatible with
  existing session files.

## Impact

duckboard-only; no library (`duckpond`) changes.

```
reload_and_reconcile ──► exploration/promotion ──► chat/persistence
 (new-change detect)      (who owns it, if anyone)   (merge, don't overwrite)
        │                          │                          │
        ▼                          ▼                          ▼
   distinguish real          require authoritative      atomic + eager writes,
   creation from VCS         binding; no focus guess    non-clobbering migration
   reappearance
```

- Touches `chat_store.rs` (atomic `save_session`, non-clobbering `rename_scope`),
  `main.rs` (`reload_and_reconcile`, `promote_idea_exploration`,
  `fallback_exploration_id`), `area/change.rs` (`promote_exploration`), and
  `area/interaction.rs` (persistence timing).

- No on-disk format change and no migration: existing sessions load unchanged.

- No breaking API changes; behavior is internal to duckboard.
