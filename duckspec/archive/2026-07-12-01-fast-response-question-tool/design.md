# Fast response and structured questions - Design

Rename the empty option-chip shell to **fast response**, then wire mid-turn structured
agent questions for Claude and Grok through the shared ACP client into those chips (⌘1…⌘n
/ ⌘⌫), answered as RPC results so the same turn continues.

## Approach

```
                    duckboard
           FastResponse chips (⌘n / ⌘⌫)
                      │
                      │ answer RPC (not new user msg)
                      ▼
              duckchat AgentHandle
                      │
         AgentEvent::UserChoiceRequest
         AgentCommand::AnswerUserChoice
                      │
                      ▼
                 AcpTurn (shared)
        mid-prompt agent→client routing
                      │
         ┌────────────┴────────────┐
         │                         │
   Grok native ACP          duckchat-claude-acp
   ask_user_question        translate Claude Q
   (drop --no-ask-user)     → choice request
   keep --always-approve    keep tool bypass
```

```
turn open
   │
   ▼
agent needs structured choice
   │
   ├─ tool permission (allow/deny kinds) ──▶ auto-select allow (no UI)
   │
   └─ question / product options ──▶ UserChoiceRequest
              │
              ▼
        is_awaiting_user = true
        fast_response = options + cancel
              │
        ┌─────┴─────┐
        ▼           ▼
      ⌘n option   ⌘⌫ cancel
        │           │
        └─────┬─────┘
              ▼
     write JSON-RPC result for pending id
     clear fast_response; awaiting = false
     turn continues → TurnComplete
```

Authority split (unchanged except chips):

```
| Surface                    | Job                                                    |
|----------------------------|--------------------------------------------------------|
| Next-card ghost / Tab      | Post-turn handoffs / write-gate tokens                 |
| Oneshot under-input ⇧↩     | Freeform reply suggestion (not chips this change)      |
| Fast-response chips        | Mid-turn structured choice (questions now; other later)|
```

## Rename (obvious → fast response)

Mechanical product rename so the shell is source-neutral.

```
caps/chat/obvious-bubble/  →  caps/chat/fast-response/
crates/duckboard/src/obvious_bubble.rs  →  fast_response.rs
ObviousChrome  →  FastResponse
obvious_chrome →  fast_response
refresh_obvious_chrome / build_obvious_chrome → refresh/build or set_fast_response
chat_obvious_chip_* → chat_fast_response_chip_*
SendObviousAction → ActivateFastResponse (or similar)
@spec chat/obvious-bubble → chat/fast-response
```

Archive history stays. Session-scope prose that says “obvious chrome” is rewritten to
lifecycle-only wording (no false coupling to chips).

Empty-session lifecycle seed for next-card bootstrap keeps using format helpers if needed;
those are **not** a fast-response population source.

## Shared ACP choice loop (`duckchat`)

Today every agent→client request is `answer_request(null)` in `AcpTurn::request`. Split by
method/kind:

```rust
// crates/duckchat/src/event.rs (sketch)
pub enum AgentEvent {
    // …existing…
    /// Mid-turn structured choice. Host must AnswerUserChoice or cancel ends it.
    UserChoiceRequest(UserChoiceRequest),
}

pub struct UserChoiceRequest {
    pub correlation_id: u64, // host-local; maps to JSON-RPC id inside worker
    pub prompt: Option<String>, // question text when known
    pub options: Vec<UserChoiceOption>, // 1..=9 shown
    pub allow_cancel: bool,
}

pub struct UserChoiceOption {
    pub id: String,    // wire optionId / answer key
    pub label: String, // chip label
}

pub enum UserChoiceAnswer {
    Selected { option_id: String },
    Cancelled,
}
```

```rust
// AgentHandle / AgentCommand
AnswerUserChoice {
    correlation_id: u64,
    answer: UserChoiceAnswer,
}
```

**AcpTurn routing** while waiting on `session/prompt` (and nested client requests):

```
| Incoming agent→client                                              | Action                                      |
|--------------------------------------------------------------------|---------------------------------------------|
| `session/request_permission` with only permission kinds            | Auto-reply selected allow_once (no UI)      |
| (`allow_*` / `reject_*`)                                           |                                             |
| `x.ai/ask_user_question` (Grok)                                    | Emit `UserChoiceRequest`; park; reply       |
|                                                                    | `AskUserQuestionExtResponse`                |
| `session/request_permission` with product option labels            | Emit `UserChoiceRequest`; park; reply       |
| (Claude adapter / other agents)                                    | permission `selected` outcome               |
| Unknown method                                                     | Safe cancel/null as today (no deadlock)     |
```

Parking: oneshot (or map of correlation_id → oneshot) owned by the worker/turn; `try_send`
events as today; cancel token completes pending choices as `Cancelled`.

**Important:** oneshot path and headless tests keep auto-null / auto-allow so they never
hang.

## Fast-response shell (`duckboard`)

```rust
// crates/duckboard/src/fast_response.rs
pub struct FastResponse {
    pub options: Vec<FastResponseOption>, // id + label; max 9 for ⌘1…⌘9
    pub cancel: Option<FastResponseOption>, // ⌘⌫ when set
    pub source: FastResponseSource,
}

pub enum FastResponseSource {
    /// Answer via AgentHandle — not send_prompt_text
    UserChoice { correlation_id: u64 },
    // later: OneshotHint / SendText { text }
}

pub fn visible(
    is_streaming: bool,
    is_awaiting_user: bool,
    input_empty: bool,
    fr: &FastResponse,
) -> bool { /* non-empty && input_empty && (!is_streaming || is_awaiting_user) */ }
```

```
AgentSession
  fast_response: FastResponse          // empty default
  is_awaiting_user: bool               // set on UserChoiceRequest; clear on answer/cancel/turn end
```

Activation path:

- **UserChoice source** → `handle.answer_user_choice(...)`; optional quiet transcript note
  if useful; **no** new user bubble via `send_prompt_text`

- Visibility while awaiting even though `is_streaming` remains true for the open turn

- Disk/lifecycle `build_*` stays empty; only live events fill options

- Refresh must **not** clobber an in-flight question (refresh only when
  `!is_awaiting_user` or merge carefully)

## Grok enablement

```
// crates/duckchat/src/grok.rs launch
// today:  grok --no-ask-user agent --always-approve stdio
// ship:   grok agent --always-approve stdio
//         (keep always-approve; drop --no-ask-user only)
```

**Wire (resolved):** questions are **not** answered via stock `session/request_permission`
alone. Observed ACP flow from a live session:

```
session/update tool_call (title: ask_user_question, rawInput.questions[…])
session/request_permission  →  auto-allow tool (always-approve; wait_ms ≈ 0)
x.ai/ask_user_question      →  client must return AskUserQuestionExtResponse
session/update tool_call_update failed if result is null
```

Method name: **`x.ai/ask_user_question`** (xAI ACP extension).

Request body carries the questionnaire (same shape as tool `rawInput`): `questions[]` with
`question`, `options[{label, description}]`, optional `multiSelect`. Tool meta marks
`kind: "ask_user"`.

Response is an **internally tagged** `AskUserQuestionExtResponse`:

```
| Variant          | Role                                      | Host mapping      |
|------------------|-------------------------------------------|-------------------|
| Accepted         | answers (+ optional partial_answers)      | ⌘n selection(s)   |
| SkipInterview    | dismiss / skip the questionnaire          | ⌘⌫ cancel         |
| ChatAboutThis    | freeform divert (not v1 chips)            | ignore / later    |
```

`Accepted` fields include `answers` and `partial_answers` (binary: “Accepted with 2
elements”). Map chip picks into `answers` keyed like Claude (question text → option label)
once encoder tests lock exact JSON tag names (`Accepted` vs `accepted`, etc.).

Client null → known failure: `expected internally tagged enum AskUserQuestionExtResponse`.

Still auto-allow **tool** `session/request_permission` (allow/reject kinds). Only
`x.ai/ask_user_question` (and any product-labeled permission that is not allow/reject)
emits `UserChoiceRequest`.

v1: **one question at a time** for chips (first unanswered / sequential if agent re-asks);
multi_select and ChatAboutThis freeform stay non-goals.

Encoder/decoder live in duckchat next to AcpTurn so duckboard never sees Grok types.

## Claude adapter enablement (`duckchat-claude-acp`)

Today the adapter is half-duplex on the ACP side: parent stdin only between methods;
during `session/prompt` it only **writes** `session/update`. Claude never gets
AskUserQuestion (`DISALLOWED_TOOLS`).

**Wire (resolved):** official Agent SDK answers questions through **`canUseTool`**, not by
posting a user `tool_result` content block. For `toolName == "AskUserQuestion"`:

```
allow + updatedInput: {
  questions: <pass-through>,
  answers: { "<question text>": "<selected option label>", ... }
}
```

Deny/cancel is a deny (or skip) with message — not a freeform user chat turn.
`AskUserQuestion` still reaches `canUseTool` even under allow/bypass for other tools
(requires user interaction). Stream-json hosts route this via the **control protocol**
(`control_request` / `sdk_control_request` permission subtype, or equivalent
`--permission-prompt-tool stdio` path) with `behavior: allow|deny` and `updatedInput` for
answers.

```
Parent AcpTurn  ←full-duplex ACP→  adapter  ←stream-json + control→  claude
                     │
                     │ mid-prompt: can_use_tool / permission
                     │ control for AskUserQuestion
                     │ → session/request_permission (or neutral
                     │   choice) to parent
                     ▼
              parent chips → adapter control_response
              { behavior: allow, updatedInput: { questions, answers } }
```

Work:

1. Remove `AskUserQuestion` from `DISALLOWED_TOOLS`; keep plan/cron/etc. disallowed; keep
   `bypassPermissions` for ordinary tools (questions still prompt via can_use_tool rules)

2. Enable control / permission-prompt path on Claude spawn so AskUserQuestion is not stuck
   without a host callback (stdio control, not interactive TTY)

3. On AskUserQuestion control/tool request: emit profile `tool_call` as today; **issue
   agent→client choice** to parent with options from tool input (v1 sequential first
   question if multi)

4. Adapter main loop accepts **responses to its own request ids** while a prompt is open

5. Map parent `Selected` → `allow` + `answers` map (question text → option label); map
   `Cancelled` → deny/skip with a short message; continue until Claude `result`

Parent still only sees neutral ACP choice; Claude control details stay in the adapter.

## Caps / docs impact (intent for later specs)

```
caps/chat/
  obvious-bubble/  →  fast-response/   (rename + awaiting-user visibility + RPC activation)
caps/harness/
  acp-client/      modify  (choice routing; auto-allow tool perms; no null for questions)
  grok/            modify  (launch flags; question path)
  claude/          modify  (AskUserQuestion allowed; adapter mid-prompt choice)
```

No new cap required if acp-client owns the neutral choice contract; optional thin
`harness/user-choice` only if specs get crowded.

## Impact

- `duckchat` public `AgentEvent` / `AgentCommand` surface grows (duckboard + any tests)
- `duckchat-claude-acp` stdout/stdin concurrency during prompts
- Grok launch argv change (tests asserting `--no-ask-user` update)
- Capability path rename + all `@spec chat/obvious-bubble` backlinks
- Theme/helper renames in duckboard

## Decisions

- **RPC answer, not user message** — matches ACP/Grok typed responses; chips for questions
  never call `send_prompt_text`. Alternative: fake user bubble (rejected: wrong channel,
  breaks turn state).

- **Auto-allow tool permissions; UI only for questions** — proposal non-goal. Alternative:
  one chrome for both (rejected: noise, fights always-approve/bypass product).

- **Neutral `UserChoiceRequest` in duckchat** — harness extensions decode at the edge.
  Alternative: duckboard parses raw ACP (rejected: leaks Grok/Claude into UI).

- **Rename in same change as first population source** — avoids shipping empty shell under
  obsolete name. Alternative: rename-only PR first (rejected: extra ceremony for a dead
  name).

- **v1 sequential single-select** — multi-Q / multi_select / freeform later. Alternative:
  full interview UI (rejected: scope).

- **Claude adapter originates ACP choice** — parent stays agent-agnostic. Alternative:
  parent parses Claude stream-json (rejected: undoes uniform ACP).

- **Grok answers via `x.ai/ask_user_question`** — not stock permission outcome for the
  questionnaire. Alternative: only handle `session/request_permission` (rejected: live
  failure shows null permission result is not enough).

- **Claude answers via canUseTool / control allow+answers** — not a synthetic user
  `tool_result` message. Alternative: inject tool_result as user content (rejected: SDK
  contract is permission allow with `updatedInput.answers`).

## Risks

- **Exact JSON tag casing for `AskUserQuestionExtResponse`** → unit-test encoder against
  Grok’s deserializer (or a scripted peer) before ship; variants are known (`Accepted` /
  `SkipInterview` / `ChatAboutThis`).

- **Claude control flag surface** → confirm spawn flags (`--permission-prompt-tool stdio`
  or current CLI equivalent) during adapter step; if control cannot open while
  `bypassPermissions` is set, narrow bypass or use dontAsk+allow rules for non-question
  tools.

- **Adapter mid-prompt duplex races** → single-threaded pump: Claude lines and parent
  lines on one select loop; never two writers to Claude stdin.

- **Refresh clobbering live chips** → gate refresh when `is_awaiting_user`.

- **Stale chips after cancel/error** → clear on `TurnComplete`, `Error`, `ProcessExited`,
  cancel.

## Open questions

None outstanding. Wire shapes for Grok (`x.ai/ask_user_question` +
`AskUserQuestionExtResponse`) and Claude (canUseTool / control allow with
`updatedInput.answers`) are recorded above.
