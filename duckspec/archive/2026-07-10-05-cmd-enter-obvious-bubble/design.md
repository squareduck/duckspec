# Cmd-Enter obvious bubble — Design

Split lifecycle next-step onto a ghost transcript bubble and ⌘↩; make composer empty-input
defaults oneshot-parse-only; flip steps-complete next stage to `ds-archive`; reword
template handoffs to a flat “Suggested next actions” list.

## Approach

```
disk artifacts
      │
      ▼
change_scope_facts.next_command
      │
      ├──────────────────────────────┐
      ▼                              │
obvious_command (session)            │
      │                              │
      │  soft hint only              │
      ├──────────────────▶ oneshot request
      │                         │
      │                         ▼
      │              parse REPLY: lines
      │                         │
      │                         ▼
      │              agent_default_prompts
      │              (parse only; empty on fail)
      │                         │
      │                         ▼
      │              composer list + empty Enter + Tab
      │
      ▼
bubble_visible? (idle, empty input, Some)
      │
      yes ──▶ ghost user chrome + ⌘↩ / click
      │              │
      │              ▼
      │         send_prompt_text(formatted command)
      │
      no ──▶ no ghost, ⌘↩ unbound for this path
```

One field stays authoritative: `AgentSession.obvious_command`, refreshed from
`change_scope_facts` (and exploration → `ds-explore`). It no longer seeds or backs the
composer list. The bubble path reads only that field; the oneshot path may still receive
it as a soft request hint and may invent other replies.

Send path is shared: bubble activation and ⌘↩ call the same `send_prompt_text` as typing
`/ds-…` and Enter. The ghost is never stored in `ChatSession.messages`; after send it
appears as a normal user bubble.

## Lifecycle ladder (session/scope)

One-line product change in `change_scope_facts` when all steps are done:

```rust
// crates/duckboard/src/area/change.rs
next_command: Some(if all_done { "ds-archive" } else { "ds-apply" }.into()),
```

Today `all_done` yields `"ds-review"`. After this change it yields `"ds-archive"`. That
flows to:

- `obvious_command` / ghost + ⌘↩
- first-turn orientation (“Suggested next stage: /ds-archive”)
- oneshot soft hint

Agent templates may still rank review above archive in handoff prose; that is
intentionally separate from the disk ladder.

Update the lifecycle table in `duckspec/caps/session/scope/doc.md` (and the matching delta
under the change) so “all steps complete” maps to archive. Adjust unit tests in
`area/change.rs` that assert `ds-review` on all-done.

Exploration and early ladder arms are unchanged:

```
exploration              → ds-explore
empty change             → ds-propose
proposal only            → ds-design
design, no caps          → ds-spec
caps, no steps           → ds-step
steps incomplete         → ds-apply
all steps complete       → ds-archive   ← changed
```

## LLM-only default prompts

`crates/duckboard/src/default_prompts.rs` stops treating the lifecycle heuristic as a list
entry.

```rust
/// Effective empty-composer defaults: settled oneshot parse only (order
/// preserved). Empty when no non-empty parse is armed.
pub fn effective_prompts(oneshot_replies: &[String]) -> Vec<String> {
    oneshot_replies
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

pub fn apply_oneshot_if_current(
    session_gen: u64,
    result_gen: u64,
    result: Result<Vec<String>, String>,
) -> Option<Vec<String>> {
    if result_gen != session_gen {
        return None;
    }
    Some(match result {
        Ok(list) => effective_prompts(&list),
        Err(_) => Vec::new(),
    })
}
```

`heuristic_as_prompts` remains (or moves next to bubble helpers) solely to format
`obvious_command` for send/display (`ds-explore` → `/ds-explore`). It is not used when
building `agent_default_prompts`.

`refresh_obvious_command` still sets `obvious_command` and `scope_facts`, but must **not**
seed `agent_default_prompts` from the heuristic. New sessions start with an empty ready
list until a non-empty oneshot settles.

Oneshot request construction keeps `lifecycle_heuristic` / `obvious_command` as a soft
hint (`ReplySuggestionRequest`) — request framing only, not list population.

Call sites in `area/interaction.rs` that pass `obvious_command` into `effective_prompts` /
empty-submit helpers drop that argument.

Spec impact on `chat/default-prompts`: replace “pre-oneshot / fail fallback = heuristic”
requirements with “list empty until non-empty parse”; keep readiness (pending → loading,
streaming hides chrome) and empty Enter / Tab on the list alone.

## Obvious bubble (chat/obvious-bubble)

### Visibility (pure)

```rust
// Prefer a small pure module, e.g. crates/duckboard/src/obvious_bubble.rs
// or helpers colocated with default_prompts.

pub fn bubble_visible(
    is_streaming: bool,
    input_empty: bool,
    obvious_command: Option<&str>,
) -> bool {
    !is_streaming
        && input_empty
        && obvious_command.map(str::trim).is_some_and(|s| !s.is_empty())
}

pub fn bubble_send_text(obvious_command: Option<&str>) -> Option<String> {
    // empty-send form with leading `/`
    todo!()
}
```

Gates match the proposal: non-streaming (awaiting user reply), empty composer, command
present. Caps / codex / archived scopes leave `obvious_command` as `None` → no bubble.
Typing hides the bubble (same as defaults chrome).

Oneshot pending does **not** hide the bubble — that is the speed path while the composer
list shows loading or stays empty.

### View chrome

In `widget/agent_chat.rs`, after real transcript segments and above the composer column,
when `bubble_visible`:

```
… last assistant answer …

┌ greyed faux user bubble ─────────────────────┐
│  /ds-archive                        ⌘↩       │
└──────────────────────────────────────────────┘

composer
  [ optional oneshot list under input ]
```

- Not a `TranscriptSeg` and not appended to `session.messages`.

- Style: muted/greyed user-bubble chrome (reuse user bubble layout tokens at reduced
  opacity or `theme::text_muted()`).

- Trailing ⌘↩ (or platform-appropriate “⌘↩”) as discoverability; optional clickable target
  on the whole bubble.

- New message: `Msg::SendObvious` (name flexible) on click.

### Keybind and send

```rust
// agent_chat::Msg
SendObvious,

// interaction handler
agent_chat::Msg::SendObvious => {
    if let Some(text) = obvious_bubble::bubble_send_text(ax.obvious_command.as_deref()) {
        if bubble_visible(ax.session.is_streaming, ax.chat_input.text().trim().is_empty(),
                          ax.obvious_command.as_deref())
        {
            send_prompt_text(ax, text, highlighter);
        }
    }
}
```

⌘↩: handle in the existing keyboard path (chat-focused), only when `bubble_visible` for
the active session. Prefer a dedicated branch over overloading `SendPressed` so empty
Enter never gains heuristic semantics.

```
⌘↩  ──visible?──▶ send_prompt_text(bubble_send_text)
plain Enter empty ──▶ empty_submit_text(list only)  // unchanged gates
```

`send_prompt_text` already clears default prompts and appends a real user message — no
special-case persistence for the ghost.

### Relationship to defaults chrome

```
| State | Bubble | Composer defaults |
|-------|--------|-------------------|
| Streaming | hidden | hidden |
| Input non-empty | hidden | hidden |
| Idle, no obvious_command | hidden | oneshot list or empty |
| Idle, obvious set, oneshot pending | **shown** | loading |
| Idle, obvious set, oneshot empty/fail | **shown** | no list |
| Idle, obvious set, oneshot with REPLY: | **shown** | list (may differ from bubble text) |
```

## Template handoffs

Text-only edits under `crates/duckspec/content/templates/*.md` (and any instructions that
tell agents to emit `**Primary**` / `**Secondary**`).

Target shape:

```markdown
Suggested next actions:

- `/ds-design` — default after a proposal
- `/ds-spec` — when skipping design is appropriate
```

Rules preserved:

- At most two ranked actions (list order = rank)

- Offer once; drop if declined

- Stage matrix unchanged (e.g. apply still prefers review in prose when steps complete;
  archive may remain secondary in handoff text even though the disk ladder’s
  `obvious_command` is archive)

Also update the explore template’s instruction prose that literally says “(**Primary**,
then **Secondary** if any)” so agents emit the new shape.

No new capability; no `ds check` schema change.

## Module / file map

```
crates/duckboard/src/
├── area/change.rs          # ladder: all_done → ds-archive
├── default_prompts.rs      # parse-only effective list; drop heuristic seed API
├── obvious_bubble.rs       # NEW (or helpers in default_prompts) — visibility + format
├── widget/agent_chat.rs    # ghost view + Msg::SendObvious
├── area/interaction.rs     # SendObvious handler; stop passing heuristic into list
└── main.rs                 # ⌘↩ when bubble visible (if not handled in widget)

crates/duckspec/content/templates/*.md   # handoff wording

duckspec/changes/…/caps/
├── chat/default-prompts/   # delta: list = parse only
├── chat/obvious-bubble/    # NEW spec + doc
└── session/scope/          # delta: steps complete → archive
```

## Decisions

- **Single `obvious_command` field** — no separate agent vs hotkey maps. Only the
  steps-complete value changes to archive. Alternatives: dual functions (rejected: agents
  already treat the ladder as soft; dual maps add drift).

- **Ghost is view chrome, not history** — not in `ChatSession.messages` until send.
  Alternatives: inject a marked ephemeral message (rejected: pollutes persistence and
  transcript rebuild).

- **⌘↩ only when bubble visible** — idle + empty composer + Some command. Alternatives:
  always bind ⌘↩ to obvious when present even while typing (rejected: fights an
  in-progress draft).

- **Empty Enter never sends the heuristic** — even while oneshot pending. Alternatives:
  pending empty Enter sends heuristic (rejected: proposal out of scope; keeps Enter =
  list-only).

- **Oneshot soft hint still receives the ladder** — including archive after steps
  complete. Alternatives: pass review as soft hint while bubble shows archive (rejected:
  reintroduces dual maps).

## Risks

- **Users expect review before archive on ⌘↩ after steps complete** → templates and
  oneshot can still surface `/ds-review` in the composer list; bubble text is explicitly
  finalize. Mitigate with clear ghost label (the slash command itself) and review
  remaining available via list/type.

- **New sessions have no composer defaults until first oneshot** → bubble carries
  `/ds-explore` (or phase command) so empty chats stay one keystroke from the lifecycle
  path; no regress vs pre-oneshot heuristic list for the structural case.

- **⌘↩ collision with other macOS/app bindings** → gate strictly on bubble visibility and
  chat focus; if a conflict appears, document and prefer the bubble click path.

## Open questions

None. Visibility gates, ladder value, LLM-only list, and single-field model were settled
in exploration and the proposal.
