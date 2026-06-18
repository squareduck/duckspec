# Session scope orientation — Design

Reuse the existing phase ladder to produce richer change facts, carry them to send time
the way `obvious_command` already travels, render them into the scope blurb, and deliver
that blurb on the reliable priming-turn channel regardless of whether `AGENTS.md` exists.

## Approach

```
  area/change.rs            area/interaction.rs           scope.rs
  ┌────────────────┐  facts  ┌─────────────────┐  build  ┌────────────────┐
  │ change_scope_  │────────→│ AgentSession    │────────→│ CurrentScope   │
  │ facts(ladder)  │ on each │ .scope_facts    │ Session │ Hook.compute   │
  └────────────────┘ session └─────────────────┘ Scope   └───────┬────────┘
        ▲                            │                           │ blurb text
        │ obvious_command            │ send_prompt_text          ▼
        │ (thin caller)              │ builds combined priming body:
        └────────────────────────────  [AGENTS.md?] + scope blurb + path note
                                       sent as the standalone priming turn
                                       (system_additions emptied)
```

The phase/next-stage ladder already exists as `obvious_command_from_artifacts` and its
result already rides every session as `ax.obvious_command`, refreshed by
`refresh_obvious_command`. The change widens that ladder to emit a facts struct,
piggybacks the facts onto the same per-session carrier, and feeds them into the scope
hook. Delivery moves from the flaky `--append-system-prompt` channel onto the priming
message body — generalizing the priming turn so it fires whenever there is *any*
orientation content, not only when `AGENTS.md` is present.

Only the new-session priming path changes. The legacy no-session-id fallback path keeps
its current `system_additions` behavior, per the proposal.

## ChangeScopeFacts — single phase/progress source

The ladder in `obvious_command_from_artifacts` is the only place that knows how artifact
presence maps to lifecycle stage. Widen it to return facts instead of just a command
string, so the blurb and the placeholder command share one source of truth.

```rust
// area/change.rs
pub struct ChangeScopeFacts {
    /// Human phase label, e.g. "specs drafted, steps not yet written".
    pub phase: &'static str,
    /// Step-level progress. `step_count == 0` means no steps yet.
    pub steps_done: usize,
    pub step_count: usize,
    /// Task tally for the one in-progress (Partial) step, if any.
    /// `StepCompletion::Done` drops totals, so a full task aggregate is not
    /// recoverable without changing the data model — out of scope here.
    pub active_step_tasks: Option<(usize, usize)>,
    /// Suggested next `/ds-*` command (without leading slash).
    pub next_command: Option<String>,
}

/// Walks the same artifact ladder as before, now returning full facts.
pub fn change_scope_facts(name: &str, project: &ProjectData) -> Option<ChangeScopeFacts> { todo!() }

/// Thin caller — preserves the existing placeholder behavior.
fn obvious_command_from_artifacts(name: &str, project: &ProjectData) -> Option<String> {
    change_scope_facts(name, project).and_then(|f| f.next_command)
}
```

## Carrying facts to send time

`send_prompt_text` builds the blurb but only has `&mut AgentSession` — no `ProjectData`.
Rather than thread project data into the send path, stash the facts on the session the
same way `obvious_command` already is.

```rust
// area/interaction.rs — AgentSession
pub scope_facts: Option<crate::area::change::ChangeScopeFacts>,

// area/change.rs — refresh_obvious_command already iterates (scope, project)
for ax in ix.sessions.iter_mut() {
    ax.obvious_command = facts.as_ref().and_then(|f| f.next_command.clone());
    ax.scope_facts = facts.clone();   // facts derived once per change scope
}
```

`ChangeScopeFacts` derives `Clone`. Non-change scopes leave `scope_facts` at `None`.

## Enriched CurrentScopeHook

`SessionScope` gains the optional facts; the `Change` arm renders name + phase + progress
+ next stage. Other arms are unchanged. The blurb is authoritative — it tells the agent
this is the change to act on unless told otherwise.

```rust
// scope.rs
pub struct SessionScope {
    pub kind: ScopeKind,
    pub scope_key: String,
    pub change_facts: Option<crate::area::change::ChangeScopeFacts>,
}

// CurrentScopeHook::compute, Change arm (sketch of the rendered shape):
//   Current duckspec scope: change `foo`. Artifacts live under `changes/foo/`.
//   Phase: specs drafted, steps not yet written (2 of 3 steps done).
//   Suggested next stage: /ds-step. Commands like /ds-archive and /ds-apply
//   act on THIS change by default — only disambiguate via `ds status` if asked
//   about a different one.
```

## Generalized first-turn priming

Today the priming turn is gated on `AgentsMarkdownHook` returning `Some`
(`interaction.rs:1110`); when `AGENTS.md` is absent there is no priming turn and the scope
blurb falls back to flaky `system_additions`. Generalize: assemble a combined priming body
and prime whenever it is non-empty.

```rust
// send_prompt_text, new-session branch (claude_session_id.is_none() && messages.is_empty())
let mut priming_parts = Vec::new();
if let Some(out) = AgentsMarkdownHook.compute(&working_dir) { priming_parts.push(out.text); }
if let Some(out) = CurrentScopeHook.compute(&scope)        { priming_parts.push(out.text); }
priming_parts.push(PATH_REFERENCE_NOTE.to_string());

if !priming_parts.is_empty() {
    let priming_text = format!(
        "{}\n\nDo not respond to this message — reply with a single dot \
         (\".\") and wait for my actual instructions.",
        priming_parts.join("\n\n"),
    );
    // ... push priming ChatMessage, stash pending_followup_prompt = text ...
    let mut req = TurnRequest::new(priming_text, working_dir);
    // system_additions stays EMPTY — all orientation now rides the body.
    handle.send_turn(req);
    return;
}
```

The follow-up dispatch (`pending_followup_prompt`, selection attachments, idea-description
injection) is unchanged.

## Scope-aware templates

`templates/archive.md` and `templates/apply.md` (and any sibling whose Context step says
"run `ds status` to identify the change") are reworded so `ds status` is a disambiguation
fallback, not the primary identifier:

```
Before: 1. Run `ds status` to identify the change to archive.
After:  1. Act on the change named in this session's scope orientation. Only
           run `ds status` to disambiguate when no scope is given, or when the
           user names a different change.
```

This is prose content under `crates/duckspec/content/templates/`, not a `caps/`
capability.

## Decisions

- **Single phase source** — widen `change_scope_facts` and make
  `obvious_command_from_artifacts` a thin caller. Alternatives: duplicate the ladder in
  the hook (rejected: two sources drift); recompute in `send_prompt_text` (rejected: needs
  `ProjectData` threaded into the send path).

- **Step-level progress only** — report `steps_done / step_count` plus the active Partial
  step's task tally. Alternatives: full task aggregate (rejected: `StepCompletion::Done`
  drops totals; a true aggregate needs a data-model change, out of scope).

- **Generalize priming over fold-into-AGENTS-body** — prime on any orientation content.
  Alternatives: append scope blurb to AGENTS.md's body (rejected: leaves the blurb flaky
  whenever `AGENTS.md` is absent).

- **Move scope blurb AND `PATH_REFERENCE_NOTE` onto the priming body** — emptying
  `system_additions` on the new-session priming turn. Alternatives: move only the scope
  blurb (rejected: the path note shares the same flaky channel and the same orientation
  purpose — the half-fix that caused this bug).

- **New-session path only** — the legacy no-session-id fallback keeps `system_additions`,
  per the proposal's out-of-scope boundary.

## Risks

- **Extra priming round-trip for `AGENTS.md`-less projects** → the blurb is tiny and the
  priming turn is a single-dot ack; cost is negligible.

- **Facts captured at first turn only** → if the change advances mid-session the blurb
  goes stale, same as today's behavior; facts refresh on the next new session.

## Open questions

- None. Phase-label wording is a spec-level detail, resolved when speccing
  `session/scope`.
