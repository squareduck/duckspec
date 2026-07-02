# Attribute new change folders to their originating session — Design

Record which exploration session ran `ds create change` by parsing the Bash tool call as
it streams in, then have reconcile consume that binding to promote the right exploration —
falling back to the active area only when no binding exists.

## Approach

The current `reload_and_reconcile` infers ownership *after the fact* from ambient UI
state, which decouples the decision from the causal event. The fix records the binding *at
the causal moment* — when an exploration session's agent announces the `ds create change`
tool call — and consumes it when the folder appears.

```text
  ┌─ CAPTURE — ToolUse handler (main.rs:1368) ────────────────────────┐
  │  ax is the originating session (resolved by routing key)          │
  │  name == "Bash" && command matches `ds create change <arg>`       │
  │    stage (slugify(arg), ax.session.scope)  ── only if Exploration │
  │  after the ax borrow ends:                                        │
  │    state.change.pending_bindings.insert(slug, exploration_id)     │
  └───────────────────────────────────────────────────────────────────┘
                              │  (binding recorded before the command runs,
                              │   so it is always present before the folder)
                              ▼
  ┌─ CONSUME — reload_and_reconcile (main.rs:2862) ───────────────────┐
  │  new folder <name> appears on disk:                               │
  │    pending_bindings.remove(<name>)                                │
  │      Some(exp_id) ───────────► route_promotion(state, exp_id, …)  │
  │      None (out-of-band) ─────► fallback: resolve exp_id from      │
  │                                active_area's selection,           │
  │                                then route_promotion(…)            │
  └───────────────────────────────────────────────────────────────────┘
                              │
                  route_promotion dispatches on idea_path
              ┌───────────────┴────────────────┐
      idea_path == None                idea_path == Some(p)
      promote_exploration          promote_idea_exploration(Path::new(&p))
                                   + retain/save the shared list
```

Because capture happens when the model *announces* the tool call — before the Bash command
executes and creates the directory — the binding is guaranteed to be in the map before
reconcile can observe the new folder. Slugging the argument with the same rule the CLI
uses to name the folder makes the map key equal to the folder name by construction.

## Pending-binding store

A new field on `area::change::State` holds folder-slug → originating exploration id. It is
transient (never serialized) and deliberately never garbage-collected — changes are
created rarely enough that a stale entry costs nothing.

```rust
// crates/duckboard/src/area/change.rs
pub struct State {
    // … existing fields …
    /// Folder-slug → originating exploration id, recorded when an
    /// exploration session's agent runs `ds create change`. Consumed by
    /// `reload_and_reconcile` to attribute the new folder. Not persisted;
    /// not cleaned up (changes are infrequent).
    pub pending_bindings: HashMap<String, String>,
}
```

## Capture hook

Extends the existing `AgentEvent::ToolUse` arm. The arm already borrows `ax` (the
originating `AgentSession`) via `state.agent_session_mut(&key)`, so the scope is known
without touching the routing key. Only exploration-scoped sessions record a binding — a
Change-scoped session that creates a change has no exploration to promote.

The `state.change.pending_bindings` insert cannot happen while `ax` is borrowed, so the
arm stages the binding in a local and the surrounding block commits it after the borrow
ends — the same staging pattern already used for `title_task_input`.

```rust
// crates/duckboard/src/main.rs — inside the ToolUse arm
AgentEvent::ToolUse { id, name, input } => {
    if ax.scope_kind == scope::ScopeKind::Exploration
        && let Some(slug) = parse_create_change(&name, &input)
    {
        staged_binding = Some((slug, ax.session.scope.clone())); // scope == exploration id
    }
    flush_pending_text(&mut ax.session);
    ax.session.messages.push(/* … unchanged … */);
}

// after the `{ let Some(ax) = … }` block, alongside the title-task handling:
if let Some((slug, exp_id)) = staged_binding {
    state.change.pending_bindings.insert(slug, exp_id);
}
```

## Command parser

Turns a `ToolUse { name, input }` into the folder slug, or `None` when the call is not a
change-creating Bash command. The input is the tool's JSON arguments; for Bash that is
`{"command": "…"}`. The argument is slugified with the shared rule so the result matches
the directory the CLI will create.

```rust
// crates/duckboard/src/main.rs (or a small sibling module)
fn parse_create_change(name: &str, input: &str) -> Option<String> {
    if name != "Bash" {
        return None;
    }
    let command = serde_json::from_str::<serde_json::Value>(input)
        .ok()?
        .get("command")?
        .as_str()?
        .to_string();
    // Find `ds create change`, take the next shell token (quote-aware),
    // stop at a shell separator (&&, ;, |, newline). Returns None if absent.
    let arg = extract_create_change_arg(&command)?;
    Some(duckpond::slug::slugify(&arg))
}
```

Parsing covers the common `ds create change <name>` form the `ds-*` skills emit, including
a single quoted multi-word title. Anything it cannot parse yields `None`, which drops
through to the active-area fallback — a wrong parse never promotes the wrong exploration,
it just declines to bind.

## `route_promotion` — unified dispatch

Both the binding hit and the fallback resolve to an exploration id, then funnel through
one helper that picks the correct promotion by the exploration's `idea_path`. This
replaces the duplicated, precedence-ordered branches and incidentally makes idea-owned
explorations impossible to mis-route.

`exp.idea_path` stores the idea's absolute path (`main.rs:1112`), and
`promote_idea_exploration` looks the idea up by `abs_path` (`main.rs:2991`), so routing
from an exploration id resolves the same idea the old idea-first branch did.

```rust
// crates/duckboard/src/main.rs
fn route_promotion(state: &mut State, exp_id: &str, new_name: &str) {
    let root = state.project.project_root.clone();
    let idea_path = state
        .change
        .explorations
        .iter()
        .find(|e| e.id == exp_id)
        .and_then(|e| e.idea_path.clone());

    match idea_path {
        None => area::change::promote_exploration(
            &mut state.change,
            &mut state.interactions,
            exp_id,
            new_name,
            root.as_deref(),
        ),
        Some(p) => {
            promote_idea_exploration(state, Path::new(&p), new_name);
            state.change.explorations.retain(|e| e.id != exp_id);
            chat_store::save_explorations(
                &state.change.explorations,
                state.change.exploration_counter,
                root.as_deref(),
            );
        }
    }
}
```

## `reload_and_reconcile` rewrite

The promotion branch at `main.rs:2862` loses its fixed `if`/`else` precedence. It now
looks the new folder up in `pending_bindings`; on a miss it resolves an exploration id
from the active area's own selection. Either way it ends in `route_promotion`, so the
"which promote fn" decision lives in exactly one place.

```rust
// replaces main.rs:2862-2927
if let Some(new_name) = state
    .project
    .active_changes
    .iter()
    .find(|c| !old_change_names.contains(&c.name))
    .map(|c| c.name.clone())
{
    let exp_id = state
        .change
        .pending_bindings
        .remove(&new_name)
        .or_else(|| fallback_exploration_id(state)); // by active_area
    if let Some(exp_id) = exp_id {
        route_promotion(state, &exp_id, &new_name);
    }
}

/// Fallback attribution when no binding was recorded: the exploration the
/// active area currently points at, if any.
fn fallback_exploration_id(state: &State) -> Option<String> {
    match state.active_area {
        Area::Change => state
            .change
            .is_exploration_selected()
            .then(|| state.change.selected_change.clone())
            .flatten(),
        Area::Ideas => {
            let idea_path = state.ideas.selected.as_deref()?;
            let idea = state.ideas.ideas.iter().find(|i| i.abs_path == idea_path)?;
            (idea.frontmatter.change.is_none())
                .then(|| idea.frontmatter.exploration.clone())
                .flatten()
        }
        _ => None,
    }
}
```

## Decisions

- **Causal signal over ambient state** — attribute from the tool-call stream, which is the
  one signal *caused* by the session running `ds create change`. Alternatives: selection /
  streaming / focus / recency (rejected: all merely correlate with authorship and invert
  in the concurrent case the bug is about).

- **Capture at `ToolUse`, not `ToolResult`** — the `ToolUse` announcement precedes command
  execution, so the binding is present before the folder can appear, eliminating any
  reconcile-timing race; combined with shared `slugify` the key equals the folder name.
  Alternative: parse the folder name out of the `ToolResult` output
  `created changes/<name>` (rejected: races a reconcile tick that fires between folder
  creation and result processing, and couples to the CLI's output wording).

- **`active_area` fallback** — when no binding exists, attribute to the session the user
  is actually in. Alternatives: keep the old change-first precedence (rejected: it is the
  bug); pick the streaming scope (rejected: streaming ≠ creator).

- **Unify via `route_promotion` keyed on `idea_path`** — one dispatch point for both the
  binding and fallback paths. Alternative: keep the two separate branches (rejected:
  duplicated logic and the precedence trap that caused the original bug).

## Risks

- **Shell-argument parsing fragility** (quotes, compound `&&`/`;` commands, a `cd` prefix)
  → parse conservatively and return `None` on anything unrecognized; an unparsed command
  declines to bind and falls through to the active-area fallback rather than
  mis-attributing.

- **Non-Bash or out-of-band creation** (CLI-created folder, or an agent that writes the
  directory without `ds create change`) → no binding is recorded; the active-area fallback
  handles it (already in proposal scope).

## Open questions

None. `exp.idea_path` being the idea's absolute path was confirmed against `main.rs:1112`
and `main.rs:2991`.
