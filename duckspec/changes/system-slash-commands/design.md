# System slash commands - Design

Duckboard-local system slash commands (v1: `/help`), explicit slash kinds for completion
paint, discovery cleanup so Claude-only builtins are not faked on Grok, and a `//` escape
so harness help remains reachable.

## Approach

```
composer submit (text)
        │
        ▼
  parse submit slash
        │
        ├── bare system name?  ──►  local handler
        │                           • User bubble (typed text)
        │                           • Role::System notice + body
        │                           • no TurnRequest / priming / title
        │
        ├── //… escape?  ──────►  Agent path
        │                           prompt = "/…"  (one leading / stripped)
        │                           display = typed form kept in transcript
        │
        └── else  ─────────────►  existing send_prompt_text
```

Completion list is a **merge** of duckboard system registry + provider discovery, each
entry tagged with `SlashCommandKind`. Paint and (tie-break) sort use kind; fuzzy score
stays primary.

```
duckboard system registry          Provider::list_commands(root)
  kind = System                      (discovery, cleaned)
        │                                   │
        │                    ┌──────────────┴──────────────┐
        │                    │ tag Workflow if name is ds-* │
        │                    │ else Agent                   │
        │                    └──────────────┬──────────────┘
        └──────────────── merge ────────────┘
                         │
                         ▼
              Vec<SlashCommand> for completion + /help body
              (system name wins on collision)
```

## Slash kinds and model

Extend the public command type in duckchat; duckboard consumes kind for paint and local
intercept.

```rust
// crates/duckchat/src/provider.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlashCommandKind {
    /// Duckboard-local; never sent as an agent turn when bare.
    System,
    /// Duckspec workflow templates (/ds-*). Agent-bound, duckspec-owned.
    Workflow,
    /// Harness / project / plugin skills. Agent-bound as-is.
    Agent,
}

pub struct SlashCommand {
    pub name: String, // no leading '/'
    pub description: String,
    pub kind: SlashCommandKind,
}
```

Workflow tagging: name starts with `ds-` (case-sensitive as today) after discovery. System
names never come from provider discovery after merge (system wins).

## Discovery cleanup

Shared `discover_commands` in `crates/duckchat/src/claude_code/discover.rs` currently
appends Claude-interactive builtins (`clear`, `compact`, `cost`, `help`, `model`) for
every harness, including Grok.

```
// remove or gate: builtins only when the provider can honor them
// GrokProvider::list_commands must not advertise CLI-only fakes
```

System commands are **not** reintroduced via discovery. Duckboard merges its registry when
building the session’s completion list (same site that already stores discovered commands
in `main` / interaction state).

## System registry and `/help`

```rust
// crates/duckboard — fixed registry (not provider)

pub struct SystemCommand {
    pub name: &'static str,
    pub description: &'static str,
    pub kind: LocalKind, // Help for v1
}

const SYSTEM_COMMANDS: &[SystemCommand] = &[
    SystemCommand {
        name: "help",
        description: "Duckboard help (local). Agent: //help",
        kind: LocalKind::Help,
    },
];
```

### Local `/help` behavior

On bare `/help` (trim; v1 treats optional args as still local help):

1. Clear composer like a normal send

2. Append **User** message with typed text

3. Append **System** message (single `ContentBlock::Text`)

4. Persist session; **do not** set streaming, prime, follow-up, title, or consume
   tentative selection / input attachments

**System message shape:**

```text
Running system command `/help`.
For agent help (harness skill docs), use `//help`.

## System (duckboard)
- /help — …

## Workflow (duckspec → agent)
- /ds-… — …

## Agent skills (→ {harness})
- /… — …

## Escape
`//help` — send `/help` to the agent (harness skill docs).
```

- Prefix is fixed two lines (command notice + escape teach)
- Sections built from the **live** merged completion list; omit empty sections
- `{harness}` from resolved session / project model harness id (`grok`, `claude-code`)
- Sort entries by name within each section

### Submit parse sketch

```rust
// crates/duckboard/src/area/interaction.rs (or small slash module)

enum SubmitSlash {
    LocalHelp,
    Agent {
        /// Text stored on the User message
        display: String,
        /// Text used in TurnRequest.prompt
        prompt: String,
    },
}

fn parse_submit_slash(text: &str) -> SubmitSlash { /* … */ }
```

- `//help` → `Agent { display: "//help", prompt: "/help" }` so Grok skill matching still
  sees `/help`

- Only one leading extra `/` stripped for escape

- Bareness uses the same rules as `chat_store::is_bare_slash_command` for the local
  branch; escape form is bare `//token` with no further args in v1 (args: pass through as
  agent text without local intercept)

## Completion visuals

In `view_completion_col` (`crates/duckboard/src/widget/agent_chat.rs`):

```
| Kind     | `/name` color              | Description   | Optional tag |
| -------- | -------------------------- | ------------- | ------------ |
| System   | accent (or slash_system)   | text_muted    | sys          |
| Workflow | success (duckspec family)  | text_muted    | (optional)   |
| Agent    | text_primary / secondary   | text_muted    | (optional)   |
```

- **Primary cue:** color on the `/name` token

- **Secondary:** short `sys` tag for system rows (helps low-contrast / colorblind)

- **Sort:** fuzzy score first (unchanged UX); kind as **tie-break** only: System →
  Workflow → Agent

- No new transcript role: system replies stay `Role::System` / `BlockKind::System`

Composer-while-typing tint deferred.

## Escape hatch

```
| Typed    | Transcript user text | Agent prompt | Path   |
| -------- | -------------------- | ------------ | ------ |
| `/help`  | `/help`              | —            | local  |
| `//help` | `//help`             | `/help`      | agent  |
```

No harness-specific `/grok-help`. General rule: one extra leading `/` forces agent path
for an otherwise system name.

## Call-site wiring

```
send path (SendPressed / empty-submit / etc.)
  text = composer buffer
  match parse_submit_slash(&text) {
    LocalHelp => run_system_help(ax, highlighter)
    Agent { display, prompt } => {
      // existing send_prompt_text, but User bubble uses `display`
      // and TurnRequest prompt uses `prompt` (when they differ)
    }
  }
```

If today’s `send_prompt_text` always uses one string for both bubble and prompt, split
that parameter (or pass an optional prompt override) so escape can diverge.

## Capability impact

```
| Path                 | Role                                              |
| -------------------- | ------------------------------------------------- |
| chat/slash-commands  | NEW: kinds, merge, intercept, // escape, paint, /help body |
| (no harness/*)       | discovery cleanup only; no ACP / -p protocol change |
| chat/transcript      | reuse System blocks; no segment model change      |
```

## Impact

- **duckchat:** `SlashCommand` gains `kind`; `SlashCommandKind` public; discover_commands
  drops or gates Claude builtins; all call sites constructing `SlashCommand` update

- **duckboard:** system registry, submit parse, local `/help` body builder, completion
  paint, optional prompt/display split on send

- **Session JSON:** no new fields; system messages already persist as `Role::System`

- **No** skill file or template content changes required for v1

## Decisions

- **`//` escape, not `/agent-help`** — one general collision rule; system notice teaches
  it. Alternative: discoverable `/agent-help` system command that forwards (add later if
  double-slash is invisible).

- **Three kinds (System / Workflow / Agent)** — workflow is agent-bound but
  duckspec-owned; coloring `/ds-*` like harness skills would hide ownership. Alternative:
  two kinds only (local vs agent).

- **System registry in duckboard** — local handlers are UI/session behavior, not provider
  capability. Alternative: provider-level “local commands” (rejected: Grok/Claude should
  not own duckboard help).

- **Keep User bubble for `/help`** — audit trail. Alternative: system-only (quieter,
  harder to scan history).

- **Score-first completion sort** — typing UX unchanged; kind color teaches. Alternative:
  kind-primary sections (more structure, worse fuzzy feel).

- **v1 system surface = `/help` only** — clear/model/cost wait for real handlers; do not
  re-list unimplemented builtins.

## Risks

- **Grok skill match may require exact `/help` prompt** → escape path sets `prompt` to
  `/help` while transcript keeps `//help`.

- **Users never discover `//`** → completion description and help prefix both teach
  escape; optional later `/agent-help` alias.

- **Workflow vs agent mis-tag if non-ds duckspec commands appear** → v1 rule is `ds-`
  prefix; revisit if naming spreads.

- **Removing Claude builtins from discovery breaks muscle memory on Claude** → only re-add
  when a real local or harness-backed handler exists; Claude interactive CLI outside
  duckboard is unchanged.

## Open questions

- None locked for approach: escape spelling, three kinds, and `/help`-only v1 are decided
  above. Implement-time: confirm Grok skill trigger text once with a live `//help` probe.
