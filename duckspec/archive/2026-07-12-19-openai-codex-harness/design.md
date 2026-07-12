# OpenAI Codex harness - Design

Own a Claude-shaped ACP agent over `codex app-server`, a thin `openai-codex` provider in
duckchat/duckboard, and `ds init codex` that installs stage skills under `.agents/skills`.

## Approach

```
duckboard  harness id "openai-codex"
     │
     ▼
OpenaiCodexProvider  (thin; list_models, launch, oneshot prefs)
     │
     ▼
duckchat::acp   AcpMainRuntime / AcpOneshotRuntime
     │  AgentLaunch → duckchat-codex-acp
     ▼
duckchat-codex-acp   (owned ACP server on stdio)
     │  profile dialect only toward parent
     ▼
codex app-server   (official binary, JSON-RPC)
     │  thread/* · turn/* · model/list · …
     ▼
Codex agent runtime
```

**Rule (same as Claude):** App Server never appears in duckboard or the shared ACP client.
Only `duckchat-codex-acp` speaks it.

**Init path (orthogonal):**

```
ds init codex
     │
     ▼
stock content/commands/codex/*/SKILL.md
     │
     ▼
.agents/skills/<stage>/SKILL.md
```

Codex discovers those skills natively; no live dual-read of `.claude`.

```
| Concern | Choice |
| --- | --- |
| Host wire | Shared ACP client only |
| Agent binary | New workspace crate `duckchat-codex-acp` |
| Backend | Official `codex app-server` (stdio) |
| Harness id (chat/catalog) | `openai-codex` |
| Init name | `codex` → `.agents/skills/` |
| Community adapters | Not shipped |
```

## Owned ACP agent (`duckchat-codex-acp`)

Workspace member beside `duckchat-claude-acp`. Binary name `duckchat-codex-acp`. Discovery
order mirrors Claude: `DUCKCHAT_CODEX_ACP` → sibling of running exe → PATH.

```
crates/duckchat-codex-acp/
  src/
    main.rs           # stdio ACP loop (initialize, session/*, cancel)
    agent.rs          # session table, process-hot app-server, prompt orchestration
    models.rs         # advertise from model/list + curated fallback
    codex/
      spawn.rs        # resolve `codex`, spawn app-server
      app_server.rs   # line JSON-RPC client over child stdio
      map.rs          # item/* / tokenUsage → profile session/update
      content.rs      # ACP prompt blocks → turn/start input (see image mapping)
      ask_user.rs     # tool/requestUserInput ↔ parent permission choice
```

**Image mapping (`content.rs`):** App Server `turn/start` input is a union of `text`,
`image` (URL), and `localImage` / `local_image` (filesystem path) — not ACP base64 blocks.
The agent maps ACP `{ type: image, mimeType, data }` by writing bytes to a per-turn temp
file and emitting `{ type: "localImage", path }`, then deleting the file when the turn
ends (or the process drops). Text blocks map 1:1 to `{ type: "text", text }`.

**ACP surface (parent-facing, fixed profile):**

```
initialize
session/new | session/load | session/prompt | session/cancel
session/update  → agent_message_chunk | agent_thought_chunk
                  | tool_call | tool_call_update (completed)
                  | _meta.totalTokens
session/request_permission  → product options for structured questions
```

**App Server subset (child-facing):**

```
initialize + initialized
model/list
thread/start | thread/resume     # ACP sessionId = thread.id
turn/start | turn/interrupt
item/* + turn/completed + thread/tokenUsage/updated
server requests: approvals (auto-allow), tool/requestUserInput (park host)
```

Session lifecycle sketch:

```
session/new(cwd, model?)
  → thread/start { cwd, model, approvalPolicy: never, … }
  → return { sessionId: thread.id }

session/load(sessionId)
  → thread/resume { threadId }
  → FS_NOT_FOUND-shaped error if missing

session/prompt(sessionId, prompt[], model?)
  → turn/start { threadId, input: mapped blocks, model? }
  → stream map → session/update
  → return stopReason; rebind sessionId only if thread id changes

session/cancel
  → turn/interrupt if in flight; kill app-server child (end heat)
```

Heat: one app-server child process-hot across main turns (Claude duplex analogy). Cancel
kills heat; next turn may spawn and resume the same thread id.

**Thread open is not deferred:** `session/new` always calls `thread/start` immediately so
the ACP `sessionId` is the real `thread.id` from open onward (no provisional id table).
Claude defers spawning the official CLI because that process is expensive; here the
expensive piece is the warm app-server *process*, not creating a thread. Deferral would
only add provisional-id bookkeeping without the Claude heat win.

Question path: translate `tool/requestUserInput` into parent `session/request_permission`
product options (selection / freeform / cancel) so the shared client’s existing
host-choice path works without a new host method. Ordinary allow/reject approvals complete
auto-allow inside the agent.

## Thin provider (`duckchat` + `duckboard`)

```rust
// crates/duckchat/src/openai_codex.rs
const HARNESS: &str = "openai-codex";
/// Preferred oneshot when advertised; shared pick_oneshot_model falls back otherwise.
const TITLE_MODEL: &str = "gpt-5.4-mini";

pub struct OpenaiCodexProvider { /* launch + OnceLock models */ }

impl Provider for OpenaiCodexProvider {
    fn id(&self) -> &str { HARNESS }
    fn list_models(&self) -> Vec<ModelInfo> { /* ACP initialize, tag harness */ }
    fn list_commands(&self, root: &Path) -> Vec<SlashCommand> {
        // discover .agents/skills/*/SKILL.md (name + description)
    }
    fn open_main_runtime(&self, wd: &Path) -> Box<dyn MainRuntime> { /* AcpMainRuntime */ }
    fn open_oneshot_runtime(&self, wd: &Path, preferred: Option<String>)
        -> Box<dyn OneshotRuntime> { /* AcpOneshotRuntime */ }
    // title_summary / reply_suggestions: same oneshot helpers as grok/claude
}
```

Oneshot preference: host config can override; default preferred id is `gpt-5.4-mini`
(cheap/fast Codex tier). Existing `pick_oneshot_model` already falls back by substring
match then first advertised model when the preferred id is absent from the account
catalog.

Launch helper parallel to `claude_acp_launch` / `resolve_claude_acp_binary`.

Duckboard registration:

```rust
// agent.rs — catalog refresh + Harness::dispatch + drive_provider arm
Harness::OpenaiCodex => drive_provider(OpenaiCodexProvider::new(), …)
```

Ship: bundle `duckchat-codex-acp` next to `duckboard` the same way as
`duckchat-claude-acp` (`just bundle` / install).

## `ds init codex` and stock skills

Extend stock CLI content and init — not a second content system.

```
content/commands/codex/
  ds-explore/SKILL.md
  ds-propose/SKILL.md
  …  (one skill dir per stage; same stage set as claude/opencode)
```

Each `SKILL.md`:

```markdown
---
name: ds-propose
description: Draft a duckspec proposal for the active change.
---

Run `ds template propose` silently. Do not respond to the user until you have
read the full output. Then follow its instructions.
```

Init mapping:

```rust
// init.rs — third harness row; install is tree copy, not flat file write
("claude",   ".claude/commands"),
("opencode", ".opencode/commands"),
("codex",    ".agents/skills"),
```

```
ds init codex  →  .agents/skills/ds-explore/SKILL.md
                  .agents/skills/ds-propose/SKILL.md
                  …
```

`content::command_files` today assumes flat `.md` files. Extend stock content API so init
can install **skill directories** for codex (e.g. iterate `commands/codex/*/` and write
each tree under `.agents/skills/`). Claude/opencode flat install stays unchanged.

Update `caps/cli/stock-content` for the new harness row and skill layout. CLI help /
`main.rs` harness list: add `codex`.

## Capabilities (expected layout)

New:

```
caps/harness/openai-codex/   # agent over owned ACP child; models; attachments;
                             # questions; unavailability; warm oneshot prefs
```

Touched:

```
caps/cli/stock-content/      # ds init codex → .agents/skills skill trees
caps/harness/selection/      # dispatch includes openai-codex (if docs need it)
caps/harness/model-catalog/  # third harness slice (if docs enumerate harnesses)
```

Shared ACP client / warm-runtime caps stay as-is unless mapping gaps force a small
extension (prefer keep question mapping inside the agent as Claude does for
AskUserQuestion).

## Impact

- New workspace crate + binary; workspace `members` and bundle/install scripts

- `duckchat` module + provider; `duckboard` harness enum/dispatch/catalog

- `ds init` third harness; stock content under `commands/codex/`

- Spec/doc deltas for stock-content and new harness cap

- Users need official `codex` on PATH (or documented install); auth is Codex’s own
  (ChatGPT / API key)

- No change to knowledge-tree `duckspec/codex/` paths

## Decisions

- **Owned ACP agent over App Server** — not host-side App Server client, not community
  `codex-acp` at runtime. Alternatives rejected: dual wire in duckchat (maintenance
  split); npm/npx adapter (conflicts with Claude house rule and ship story).

- **Harness id `openai-codex` vs init name `codex`** — chat/persistence use `openai-codex`
  to avoid colliding with knowledge “codex”; CLI init uses short `codex` as the
  user-facing harness keyword.

- **Skills via stock SKILL.md trees** — init writes Codex-native layout; no symlink farm
  from `.claude` in v1.

- **Structured questions via `session/request_permission` product options** — reuse host
  chips; agent owns App Server ↔ permission encoding (Claude pattern). Alternative: new
  host method (rejected for v1).

- **Auto-approve ordinary tools** — `approvalPolicy: never` (or equivalent) on thread/turn
  so parity matches Claude bypass / Grok always-approve.

- **Image attach via `localImage` temp files** — App Server accepts `image` (URL) and
  `localImage` (path), not ACP base64. Agent writes pasted bytes to a temp file, sends
  `localImage`, cleans up after the turn. Alternative (data-URL on `image.url`) rejected
  as less clearly documented for app-server.

- **Oneshot preferred model `gpt-5.4-mini`** — matches current cheap/fast Codex tier for
  titles and reply suggestions; host oneshot prefs and `pick_oneshot_model` fallback cover
  accounts that do not advertise that id. Alternative: nano-only (rejected as default —
  mini is the documented high-frequency subagent tier).

- **Immediate `thread/start` on `session/new`** — session id is always the real thread id;
  no provisional open map. Alternative: Claude-style defer until first prompt (rejected —
  cost is process heat, already owned by the warm app-server child, not per-thread open).

## Risks

- **App Server schema churn** → pin against documented methods; generate/capture fixtures
  from installed `codex`; map only the v1 subset.

- **`tool/requestUserInput` experimental / incomplete** → spike early; if missing,
  tools/usage/images still ship; question parity documented as adapter-dependent.

- **Token usage shape differs** → agent must normalize to `_meta.totalTokens` or usage
  meter stays empty.

- **Bundle size / second agent binary** → same packaging path as Claude agent; acceptable.

- **Skill discovery in composer** → provider `list_commands` must parse `.agents/skills`
  so duckboard slash UI matches what Codex loads.

- **Temp-file image lifecycle** → always delete on turn end / cancel / process drop so
  attachment bytes do not linger under the system temp dir.
