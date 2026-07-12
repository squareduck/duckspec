# Chat session scroll - Design

Centralize chat scroll policy in duckboard’s update wrapper: snap to latest when the
active chat session identity changes for an intentional open/switch; restore remembered
viewport on pure area navigation; never replay layout preservation across session identity
changes.

## Approach

Chat viewport lives on a **shared** iced scrollable (`CHAT_SCROLLABLE_ID`) while scroll
*intent* lives per `AgentSession` (`stick_to_bottom`, `last_chat_offset_y`). The
scrollable rebuilds often and defaults to y = 0. Policy belongs in
`update_with_scroll_preservation` in `crates/duckboard/src/main.rs`, using identity
before/after `update` plus a small message classifier.

```
                    ┌─ same ChatIdentity ──────────────────► layout preserve
                    │                                         (replay snapshot)
  message ──► update
                    │
                    └─ identity changed ──┬─ AreaSelected ─► restore_chat_scroll
                                          │                   (remembered viewport)
                                          └─ open/switch ──► snap_chat_to_latest
                                                              (stick + snap_to_end)
```

**ChatIdentity** = active `Scope` + active `session.id`. Captured before and after
`update`.

```
| Event | Identity | Policy |
| --- | --- | --- |
| Layout noise, same session | same | preserve (existing snapshot replay) |
| `AreaSelected` only | may change | **restore** target session memory |
| List scope pick, session tab, new/clear session, dashboard open of a change/exploration | changes (or same-area rebind) | **snap latest** |
| Find / landmarks | any | unchanged (`chat_scroll_overridden`) |
```

Incidental area entry that is not `AreaSelected` and not an open/switch classifier (e.g.
open-file path that calls `switch_area`) keeps **restore** via existing call sites, and
must **not** replay the pre-update snapshot when identity changed.

## Chat identity

```rust
// crates/duckboard/src/main.rs (private helpers)

#[derive(Clone, PartialEq, Eq)]
struct ChatIdentity {
    scope: scope::Scope,
    session_id: String,
}

fn active_chat_identity(state: &State) -> Option<ChatIdentity> {
    let scope = state.active_scope()?;
    let ax = state.interactions.get(&scope)?.active()?;
    Some(ChatIdentity {
        scope,
        session_id: ax.session.id.clone(),
    })
}
```

## Snap to latest vs restore

Reuse restore; add an explicit “open at latest” that also **writes** session intent so
streaming and later area returns stay consistent.

```rust
/// Force the active session to latest and issue snap_to_end.
fn snap_chat_to_latest(state: &mut State) -> Task<Message> {
    // active ax: stick_to_bottom = true; pending_snap_to_bottom = false
    // (optional: leave last_chat_offset_y; ChatScrolled will refresh)
    iced::widget::operation::snap_to_end(widget::agent_chat::CHAT_SCROLLABLE_ID)
}

// existing — unchanged contract
fn restore_chat_scroll(state: &State) -> Task<Message> { /* stick → end, else last y */ }
```

Mirrors `ChatLandmarkAction::HistoryBottom` for the stick flag, without inventing a second
scrollable id.

## Update wrapper policy

```rust
fn update_with_scroll_preservation(state: &mut State, message: Message) -> Task<Message> {
    let id_before = active_chat_identity(state);
    let snapshot = /* capture as today when not chat-scroll / chrome-layout */;
    let task = update(state, message);
    let id_after = active_chat_identity(state);

    if state.chat_scroll_overridden { /* unchanged early return */ }
    if has_pending_chat_autoscroll(state) { /* unchanged */ }

    let task = if id_before != id_after {
        // Never replay the previous session's snapshot onto the new one.
        if matches!(message, Message::AreaSelected(_)) {
            // update() already returns restore_chat_scroll; do not double-issue
            // unless we remove it from update and own restore only here.
            task
        } else if message_opens_or_switches_chat(&message) {
            Task::batch([task, snap_chat_to_latest(state)])
        } else {
            // Identity changed for other reasons (e.g. open-file area entry):
            // do not preserve old snapshot; prefer restore if not already issued.
            Task::batch([task, restore_chat_scroll(state)])
        }
    } else {
        match snapshot {
            Some(snap) => Task::batch([task, replay_chat_scroll(snap)]),
            None => task,
        }
    };
    // chrome pad measure as today
}
```

**`message_opens_or_switches_chat`** (name flexible) is a closed match, same style as
`is_chat_focus_msg` / `message_opens_content`:

- `Change` / `Ideas` / routed `Interaction`: `SelectSession`, `NewSession`, `ClearSession`
- `Ideas::SelectIdea`, `Ideas::StartExploration` (ends in SelectIdea)
- `Change::SelectChange`, `Change::AddExploration`
- `Change::OpenIdeaForChange`, `Ideas::OpenChange`
- Dashboard: change / archived / exploration clicked, add exploration

**Same-identity re-select** (click already-selected idea/change): no identity change →
layout preserve only; no forced snap.

**Optional cleanup:** once the wrapper owns post-identity scroll, remove redundant
`restore_chat_scroll` returns from paths that only exist to fight the rebuild *when those
paths also open a session* — replace with snap via the classifier. Keep
`restore_chat_scroll` on pure `AreaSelected` (and any path that only changes area without
being an open/switch).

## Area handlers

Area modules keep selecting/binding sessions; they do **not** each invent scroll tasks.

```
| Site | Today | After |
| --- | --- | --- |
| `ideas::open_idea` / `SelectIdea` | no scroll | identity change → wrapper snaps |
| `change::SelectChange` | no scroll (in-area) | identity change → wrapper snaps |
| `SelectSession` / `NewSession` | focus only | identity change → wrapper snaps |
| `AreaSelected` | `restore_chat_scroll` | restore; no snapshot replay across identity |
| Cross-area open + select | mixed restore | classifier → snap latest |
```

No new fields on `AgentSession`. No persistence of scroll across restarts (non-goal).

## Impact

- Duckboard-only (`crates/duckboard/src/main.rs` primarily; tiny touch points if any
  message routing needs extracting)

- No duckpond / CLI / on-disk chat format changes

- No new crates or deps

- Behavioral: session open/switch lands at bottom; area return keeps mid-history; layout
  preserve no longer bleeds across sessions

## Decisions

- **Policy in the outer wrapper, not per-area updates** — one place that already owns
  preserve vs override. Alternative: return snap/restore from every select arm (rejected:
  easy to miss, fights the wrapper’s post-update replay).

- **Force latest on every intentional open/switch, not only first process open** — matches
  proposal (“opens at latest”); remembered mid-history is only for area return to the same
  still-active session. Alternative: restore last offset when re-selecting a previously
  viewed session in-area (rejected: proposal chose latest for scope picks and session
  tabs).

- **Identity = scope + session id** — distinguishes multi-session tabs under one
  idea/change; area alone is not enough.

- **`AreaSelected` is the pure “restore” area signal** — other identity changes default to
  snap when classified as open/switch, else restore without replaying the old snapshot.

## Risks

- **Double scroll tasks** (update restores + wrapper snaps) → prefer single owner: either
  strip restore from open/switch arms or skip wrapper snap when task already set; use
  `chat_scroll_overridden` or a one-shot flag if needed.

- **Snap before layout** (content height still 0) → same as existing stream/area snap;
  iced `snap_to_end` is already used elsewhere; if first paint misses, a follow-up measure
  path may be needed (watch in implementation).

- **Misclassified messages** snap or restore wrong → keep the classifier closed and
  unit-test the pure classify + identity-diff helpers where practical.
