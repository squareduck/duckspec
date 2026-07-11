# Empty session lifecycle ghost and promotion focus - Design

Wire empty-session next-action bootstrap from scope kind (including exploration), and
re-focus the chat input after a bound exploration→change promotion remounts the chat tree.

## Approach

```
empty session
     │
     ▼
refresh_next_actions
     │
     ├─ ScopeKind::Exploration ── bootstrap "ds-explore"
     ├─ ScopeKind::Change ────── scope_facts.next_command (ladder as today)
     └─ Caps / Codex ─────────── none
     │
     ▼
next_action_list(session_empty, bootstrap, …)
     │
     ▼
composer ghost + empty Enter   (not gated by agent_input_hints)


ds create change (exploration agent)
     │
     ▼
pending_bindings + directory appears
     │
     ▼
reload_and_reconcile → promote_bound_exploration
     │
     ├─ migrate Exploration → Change (existing)
     └─ return promoted=true → Task::batch([…, focus_chat_input()])
```

Pure list building already implements empty-session seeding
(`default_prompts::next_action_list`). The miss is **bootstrap source**:
`refresh_next_actions` only reads `scope_facts.next_command`, and `refresh_obvious_chrome`
sets `scope_facts = None` for explorations. Change scopes already get the ladder via
`change_scope_facts`; exploration never gets a first lifecycle option into bootstrap.

Promotion already moves sessions and selection; file-event / refresh callers never issue
`focus_chat_input()` after that remount, unlike new-session paths.

## Empty-session bootstrap

Resolve bootstrap inside `AgentSession::refresh_next_actions` from `scope_kind` + existing
facts — do **not** invent fake `ChangeScopeFacts` for explorations (orientation must stay
non-change).

```rust
// crates/duckboard/src/area/interaction.rs — sketch
impl AgentSession {
    fn lifecycle_bootstrap(&self) -> Option<&str> {
        match self.scope_kind {
            ScopeKind::Exploration => Some("ds-explore"),
            ScopeKind::Change => self
                .scope_facts
                .as_ref()
                .and_then(|f| f.next_command.as_deref()),
            ScopeKind::Caps | ScopeKind::Codex => None,
        }
    }

    pub fn refresh_next_actions(&mut self, after_turn: bool) {
        let session_empty = self.session.messages.is_empty();
        let bootstrap = self.lifecycle_bootstrap();
        // next_action_list(session_empty, bootstrap, last_assistant…) unchanged
    }
}
```

- Empty-send form stays in `bubble_send_text` / `format_lifecycle_command` (`ds-explore` →
  `/ds-explore`).

- `agent_input_hints` remains oneshot-only (`oneshot_display_prompts`,
  `should_begin_reply_oneshot`).

- After the first message, `session_empty` is false → trailing `next` only (unchanged).

- Post-promotion: session usually **non-empty** (tool + turns); bootstrap does not apply;
  focus fix is separate.

Tests: unit on bootstrap resolution / `refresh_next_actions` with empty exploration and
empty change+open-steps; keep pure `next_action_list` scenarios.

## Promotion refocus

Thread a “did we promote?” flag out of reconcile; only **bound** promotion (existing
`pending_bindings` path) triggers refocus — not unbound new directories.

```rust
// crates/duckboard/src/main.rs — sketch
struct ReconcileOutcome {
    archived: bool,
    promoted: bool,
}

fn promote_bound_exploration(state: &mut State, new_name: &str) -> bool { /* … */ }

fn reload_and_reconcile(state: &mut State) -> ReconcileOutcome {
    // … detect new change name …
    let promoted = promote_bound_exploration(state, &new_name);
    // … archive migrate … refresh_obvious_chrome …
    ReconcileOutcome { archived, promoted }
}
```

Call sites:

```
| Caller | Today | After |
| --- | --- | --- |
| File watcher (`tree_changed`) | `if reload… { refresh tabs }` | same for `archived`; if `promoted` also `focus_chat_input()` |
| `Message::Refresh` | reload only | if `promoted`, batch `focus_chat_input()` |
```

Reuse existing `focus_chat_input()` → `operation::focus(CHAT_INPUT_ID)`.

No change to binding authority (`exploration/promotion`): still consume `pending_bindings`
only.

## Impact

- `crates/duckboard/src/area/interaction.rs` — bootstrap from `scope_kind`

- `crates/duckboard/src/main.rs` — reconcile outcome + focus tasks

- Specs (later): likely `chat/default-prompts` (who supplies first lifecycle option for
  exploration empty sessions) and `exploration/promotion` (focus continuity after bound
  promote)

- No duckpond / CLI / config / persistence schema changes

## Decisions

- **Bootstrap via `scope_kind`, not synthetic `ChangeScopeFacts`** — keeps orientation
  free of fake change phase/steps. Alternative: always-filled facts for exploration
  (rejected: wrong type and orientation coupling).

- **Hardcode exploration bootstrap as `ds-explore`** — matches historical
  `compute_obvious_command` and the only explore entry point. Alternative: full
  multi-option ladder into ghost (rejected: next-card design is single bootstrap entry).

- **Refocus on every bound promotion** — selection already follows the promoted
  exploration when it was selected; matches “keep typing after create change.”
  Alternative: only if `chat_input_focused` was true (rejected: flag is heuristic and
  often false during streaming/tool turns).

- **Return struct from `reload_and_reconcile`** — archive and promote are independent
  flags for callers. Alternative: side-effect focus inside promote (rejected: promote is
  pure-ish state mutate; Tasks belong at message handlers).

## Risks

- **Iced focus after tree remount** → same pattern as new-session / Tab cycle; if one
  frame is too early, batch with existing post-update focus style (no special delay unless
  tryout shows need).

- **Change empty-session already works in code** → still add coverage so both scopes stay
  fixed; avoid “exploration-only” patch that drifts later.
