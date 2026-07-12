# User choice transcript - Design

Host-side display and persistence for mid-turn structured questions: live question chip
above option chips, then two durable content blocks on settle (or nothing on cancel).
Agent wire stays in-band.

## Approach

```
UserChoiceRequest { prompt, options, correlation_id }
        │
        ▼
apply_user_choice_request
  · keep prompt on UserChoice shell source
  · fill options, is_awaiting_user = true
        │
        ▼
live render (scroll chrome after transcript)
  [Q chip — bg_chat_area, no ⌘, not clickable]
  [⌘1 …] [⌘2 …]   numbered accent chips
        │
   ┌────┴────────────────────┐
   │ pick / freeform         │ cancel (esc esc / turn end)
   ▼                         ▼
wire: answer_user_choice     wire: Cancelled (existing)
host: append two messages    host: clear shell only
  Assistant: UserChoiceQuestion
  User:      UserChoiceAnswer
  (labels only; no ⌘)
clear shell                  no transcript blocks
```

Boundaries:

- **duckboard only** for UI, session model, settle/commit. Harness decode already supplies
  `prompt`.

- **In-band answer** unchanged: still `AgentHandle::answer_user_choice`, never
  `send_prompt_text` for picks/freeform.

- **Host transcript ≠ agent user turn**: stored User-role answer blocks are
  display/history for the human UI (and optional recovery preamble), not a second wire
  turn.

## Content blocks (`chat_store`)

Extend `ContentBlock` in `crates/duckboard/src/chat_store.rs`:

```rust
pub enum ContentBlock {
    Text(String),
    Reasoning(String),
    ToolUse { id: String, name: String, input: String },
    ToolResult { id: String, name: String, output: String },
    /// Mid-turn question text shown as a chip (host display).
    UserChoiceQuestion { text: String },
    /// Settled pick label or freeform text shown as a chip (host display).
    UserChoiceAnswer { text: String },
}
```

Commit shape (two messages, one block each):

```rust
// Assistant — question (omit message when prompt empty/whitespace)
ChatMessage {
    role: Role::Assistant,
    content: vec![ContentBlock::UserChoiceQuestion { text }],
    …
}
// User — answer (pick label or freeform; never hotkey prefix)
ChatMessage {
    role: Role::User,
    content: vec![ContentBlock::UserChoiceAnswer { text }],
    …
}
```

Serde: new variants only appear on new writes; legacy session files load unchanged.

Consumers that match on `ContentBlock` must handle the new arms (exhaustiveness):

```
| Consumer                     | Behavior                                              |
|------------------------------|-------------------------------------------------------|
| build_transcript_segments    | Map to new TranscriptSeg variants (below)             |
| title_summarization_target   | Ignore (already Text-only) — good                     |
| build_history_preamble       | Emit labeled lines so non-resume recovery keeps Q→A   |
| Default-prompt / meta-card   | Ignore (Text-only) — good                             |
```

## Live shell (`fast_response` + `AgentSession`)

Carry prompt on the UserChoice source so oneshot fills stay prompt-free:

```rust
pub enum FastResponseSource {
    None,
    UserChoice {
        correlation_id: u64,
        prompt: Option<String>,
    },
    OneshotHints,
}

pub fn from_user_choice(
    correlation_id: u64,
    prompt: Option<String>,
    options: impl IntoIterator<Item = (String, String)>,
) -> FastResponse { … }
```

`apply_user_choice_request` stops discarding `prompt` and passes it through.
`is_awaiting_user` / clear paths unchanged in meaning.

## Settle commit (`area/interaction`)

Single helper used by pick activation and freeform-while-awaiting:

```rust
/// Resolve display answer text, append Q/A host messages when settling,
/// clear the shell. Caller still performs the wire answer.
fn settle_user_choice_transcript(ax: &mut AgentSession, answer_text: String) { … }
```

Rules:

```
| Path                              | Wire                        | Transcript                                      |
|-----------------------------------|-----------------------------|-------------------------------------------------|
| Option pick                       | Selected { option_id }      | Q (if non-empty prompt) + A with option label   |
| Freeform                          | Custom { text }             | Q (if non-empty) + A with freeform text         |
| Cancel / turn end / error clear   | existing cancel/clear       | no Q/A blocks                                   |
```

Call sites:

- `activate_fast_response` (UserChoice arm) — after wire answer, before/alongside
  `clear_user_choice_shell`

- Freeform submit in chat `SendPressed` — after wire answer

- `clear_user_choice_shell` alone (cancel, `TurnComplete` leftover, error) — **does not**
  commit

Pick label resolution: look up `option_id` in `fast_response.options`; fall back to id if
missing.

Empty / missing prompt: no question message; still commit answer on settle.

## Transcript segments + render (`widget/agent_chat`)

```rust
pub enum TranscriptSeg {
    // existing…
    UserChoiceQuestion { text: String },
    UserChoiceAnswer { text: String },
}
```

`build_transcript_segments`: map the new blocks regardless of role pairing (defensive);
clear activity coalescing like other non-tool segments.

Render:

- Shared chip chrome (padding, radius, full width) with two styles:

  - Question → new `theme::chat_fast_response_chip_question` fill = `bg_chat_area()` (+
    same soft border language as chips)

  - Answer → existing numbered/quiet-accent chip fill (`chat_fast_response_chip_numbered`
    or shared tint helper) **without** `⌘n` label

- Live shell `view_fast_response`: if `UserChoice` + non-empty prompt, paint question chip
  first (not a button), then option chips with `option_chip_label` as today

Settled and live question/answer chips share the same style helpers so reload matches the
live moment.

## Theme

```rust
/// Question chip: chip geometry, chat-area fill (agent-like), not accent-tinted.
pub fn chat_fast_response_chip_question(_theme: &Theme) -> container::Style { … }
```

Answer chips keep accent; question deliberately does not.

## Spec / capability impact

Primary capability: **`chat/fast-response`** — amend:

- Population / live UI: prompt chip above options when awaiting

- Question activation / freeform: still no **agent** user turn; **host** appends two
  blocks on settle

- Cancel: no host blocks

- Settled answer labels omit hotkeys; freeform uses answer chip style

Secondary: **`chat/persistence`** only if we add explicit round-trip scenarios for the new
blocks (otherwise covered by existing session JSON persistence once blocks serialize).
Prefer a small persistence scenario if check/audit wants an explicit backlink.

No harness cap changes (decode already correct).

## Impact

- `ContentBlock` enum growth → exhaustive matches in duckboard (compile-driven)

- Session JSON gains new block variants; old files remain loadable

- Recovery preamble gains optional Q/A lines (behavior change only when those blocks
  exist)

- Spec/doc updates under `chat/fast-response` (+ optional persistence scenario)

- Tests: unit for settle rules; render/segment mapping; amend “no user message” scenarios
  to “no agent turn / no Text user bubble for activation”

## Decisions

- **Two messages, one block each** — Assistant question + User answer. Alternatives:
  single compound block (rejected: weaker role semantics); fake `Text` bubbles (rejected:
  wrong styling and title/meta side effects).

- **Prompt on `FastResponseSource::UserChoice`** — keeps oneshot shell free of question
  state. Alternative: parallel `AgentSession` field (rejected: two sources of truth next
  to the shell).

- **Commit only on settle** — live Q is shell chrome until answer. Alternative: append Q
  at request time (rejected: cancel would need delete/tombstone; proposal wants
  remove-on-cancel).

- **Answer label = option label or freeform text** — never hotkey-prefixed when stored or
  re-rendered settled.

- **Empty prompt** — no Q chip live or settled; answer still logged on settle.

## Risks

- **Mid-turn insert while streaming continues** → Q/A land in document order before later
  assistant deltas; materialize after commit so UI updates immediately.

- **Id without matching option** → fall back to id string for answer text.

- **Exhaustive match misses** → rely on `cargo`/`clippy` for new enum arms; audit `match`
  sites in interaction + agent_chat + chat_store.
