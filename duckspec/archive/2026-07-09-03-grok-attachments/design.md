# Grok session attachments — Design

Teach the grok harness to resolve `TurnRequest` attach markers into ACP multi-block
`session/prompt` payloads, reusing Claude's link-walk semantics with ACP image encoding.

## Approach

Duckboard already fills `TurnRequest.attachments` and embeds `[label](attach:<id>)`
markers in `prompt`. Claude walks those markers; Grok currently folds only text and sends
a single ACP text block. The fix is entirely inside **duckchat**: assemble content blocks,
then put them on the wire.

```
TurnRequest
  system_additions: [scope blurb, …]
  prompt: "…[img.png](attach:id)…"
  attachments: { id → Attachment { label, media_type, bytes } }
        │
        ▼
┌───────────────────────────────────────────────────────────┐
│  duckchat attach assembly (shared walk + dual encode)     │
│                                                           │
│  1. fold system_additions + prompt into one text stream   │
│  2. walk [label](attach:<id>) spans                       │
│  3. emit neutral segments                                 │
│  4. encode → Claude Anthropic JSON  |  Grok ACP JSON      │
└───────────────────────────────────────────────────────────┘
        │                                    │
        ▼                                    ▼
  claude stream-json content          AcpTurn::prompt
  blocks (existing shape)             session/prompt.prompt: [
                                        { type: text, text },
                                        { type: image, mimeType, data },
                                        …
                                      ]
```

No duckboard changes. Selection-context chips stay out of this path (they are already
plain text prepended to `prompt`).

## Shared attach walk

Claude already implements the walk in `crates/duckchat/src/claude_code/run.rs`
(`assemble_user_content`). Extract it into a crate-private module so both harnesses share
parse rules and fallbacks; keep encoding provider-specific.

```
crates/duckchat/src/
├── attach.rs          ← NEW (crate-private)
├── claude_code/run.rs ← thin encode over attach segments
└── grok.rs            ← fold system text + encode ACP blocks
```

Neutral segments — no JSON, no base64 yet:

```rust
// crates/duckchat/src/attach.rs

pub enum Segment {
    Text(String),
    Image {
        media_type: String,
        bytes: Vec<u8>,
    },
}

/// Walk `prompt` for `[label](attach:<id>)` links.
///
/// Resolved image/* → Segment::Image
/// Resolved non-image → Segment::Text("[attachment: {label} ({n} bytes)]")
/// Unresolved / malformed / non-attach links → Segment::Text (literal span)
/// Adjacent text segments may be merged by the encoder's append helper.
pub fn walk(prompt: &str, attachments: &HashMap<String, Attachment>) -> Vec<Segment> {
    todo!()
}
```

Semantics match Claude today (and its unit tests move with the walk):

```
| Input | Output segment |
| --- | --- |
| plain text | `Text` |
| `[x](attach:id)` + image/* att | `Image { media_type, bytes }` |
| `[x](attach:id)` + non-image att | `Text("[attachment: label (N bytes)]")` |
| unresolved id / bad link | `Text` of the original span |
| empty prompt | single empty `Text` (callers still emit one block) |
```

Claude encoder (Anthropic content-block shape already on the wire):

```rust
// crates/duckchat/src/claude_code/run.rs

fn encode_anthropic(segments: &[Segment]) -> Vec<serde_json::Value> {
    // text → { "type": "text", "text": … }
    // image → { "type": "image", "source": {
    //            "type": "base64", "media_type": …, "data": <b64> } }
    todo!()
}
```

Grok / ACP encoder (MCP-style content blocks per ACP):

```rust
// crates/duckchat/src/grok.rs  (or attach.rs as encode_acp)

fn encode_acp(segments: &[Segment]) -> Vec<serde_json::Value> {
    // text → { "type": "text", "text": … }
    // image → { "type": "image", "mimeType": …, "data": <b64> }
    todo!()
}
```

`base64` is already a duckchat dependency.

## Grok content assembly

Replace string-only `assemble_prompt` with multi-block assembly used by `run_turn`.

```rust
// crates/duckchat/src/grok.rs

/// Fold system_additions ahead of the user prompt (blank-line separated, empty
/// additions dropped), then walk attach markers into ACP content blocks.
fn assemble_content(req: &TurnRequest) -> Vec<serde_json::Value> {
    let text = fold_system_and_prompt(req); // existing join logic
    let segments = crate::attach::walk(&text, &req.attachments);
    encode_acp(&segments)
}

fn fold_system_and_prompt(req: &TurnRequest) -> String {
    // same as today's assemble_prompt body
    todo!()
}
```

`run_turn` becomes:

```rust
// crates/duckchat/src/grok.rs — GrokProvider::run_turn sketch

let content = assemble_content(&req);
turn.prompt_events(
    &session_id,
    &content,
    &model,
    req.reasoning,
    context_window,
    &events,
    &cancel,
).await?;
```

Title summary keeps a one-shot text prompt (no attachments): build a single-block vec or a
small helper that wraps a string as `[{ type: text, text }]`.

## ACP prompt wire

`AcpTurn::prompt` / `prompt_events` currently hardcode a single text block. They take
content blocks instead.

```rust
// crates/duckchat/src/grok/acp.rs

pub async fn prompt(
    &mut self,
    session_id: &str,
    content: &[serde_json::Value],
    model: &str,
    reasoning: Option<ReasoningMode>,
    on_update: &mut (dyn FnMut(&Value) + Send),
    cancel: &CancelToken,
) -> Result<PromptResult, Error> {
    let mut params = json!({
        "sessionId": session_id,
        "prompt": content,
    });
    // model + reasoningEffort unchanged
    todo!()
}

pub async fn prompt_events(
    &mut self,
    session_id: &str,
    content: &[serde_json::Value],
    model: &str,
    reasoning: Option<ReasoningMode>,
    context_window: Option<usize>,
    events: &mpsc::Sender<AgentEvent>,
    cancel: &CancelToken,
) -> Result<PromptResult, Error> {
    todo!()
}
```

Wire shape for a mixed turn:

```json
{
  "method": "session/prompt",
  "params": {
    "sessionId": "…",
    "prompt": [
      { "type": "text", "text": "scope blurb…\n\nlook at " },
      { "type": "image", "mimeType": "image/png", "data": "iVBOR…" },
      { "type": "text", "text": " and tell me what you see" }
    ],
    "model": "grok-4.5"
  }
}
```

Existing ACP integration tests that script `session/prompt` keep working if they pass a
one-element text array (or the helper still accepts that).

## Testing

```
| Layer | What |
| --- | --- |
| `attach::walk` unit tests | move / mirror Claude's cases: plain, one image, two interleaved, unresolved, non-attach markdown, malformed, empty |
| `encode_acp` unit tests | image block has `mimeType` + `data`, not Anthropic `source` |
| Claude path | still green via `encode_anthropic` over the same walk (regression) |
| Grok ACP tests | `prompt` params include multi-block `prompt` array when attachments present |
```

No live grok binary required for the unit/scripted-peer tests already used under
`grok/acp.rs`.

## Decisions

- **Shared walk, private to duckchat** — extract `attach` module with neutral `Segment`s
  and dual encoders. Alternatives: (1) duplicate the walk only in grok (rejected: two
  copies of parse edge cases); (2) public capability / shared crate API (rejected:
  proposal keeps assembly internal, not a new capability).

- **Always send image blocks** — do not gate on agent `image` prompt capability in this
  change. Alternatives: negotiate via initialize `promptCapabilities` and degrade to a
  text note when missing (deferred; proposal left UI/capability gating out of scope; grok
  vision models accept images in practice).

- **Fold system_additions into the walked text stream** — same blank-line join as today,
  then walk once. Alternative: separate leading text blocks per addition (rejected: no
  behavioral gain; attach markers only appear in user prompt text).

- **No duckboard changes** — send path already sets `req.attachments` and inserts
  `attach:` links.

## Risks

- **Grok rejects image content blocks** (unknown agent, older binary) → turn fails with a
  typed process/protocol error rather than silent ignore; mitigate by keeping Claude path
  unchanged and covering encode shape with unit tests. If field names are wrong, fix
  against ACP docs (`mimeType` / `data`, not Anthropic `source`).

- **Large base64 payloads** inflate prompt size and context usage → accepted; same as
  Claude. No size limits introduced here.

## Open questions

None remaining for this design. Capability negotiation is explicitly deferred (see
Decisions).
