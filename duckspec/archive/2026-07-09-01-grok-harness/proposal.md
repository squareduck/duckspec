# Add grok as a second agent harness

Introduce grok (via `grok agent stdio` / ACP) as a second selectable harness in duckboard,
with an explicit provider dimension on model choices, harness-aware UI, and grok-4.5 as
the new default model.

## Motivation

duckboard is hardwired to a single harness. `ClaudeCodeProvider` is constructed at every
call site (model listing, worker spawn, title summary), the model list is hardcoded, there
is no real default — an unset model means "let the CLI pick" — and the UI is purely
model-based with no notion of which harness is running. The `Provider` trait in duckchat
was built for multiple backends but has only ever had one implementation.

A spike confirmed grok is a viable second harness: its ACP stdio stream delivers
everything duckboard needs — streamed content, reasoning, structured tool events, live
token usage, and the accurate 500k context window — and grok natively loads duckspec's
existing `.claude` skills. Adding it gives users a second frontier agent and finally
exercises the abstraction the codebase already committed to.

## Scope

```
duckchat/
  provider.rs ───── Provider trait already abstract; ModelInfo gains harness identity
  claude_code/ ──── existing impl, behavior untouched
  grok/  ← NEW ──── GrokProvider: ACP JSON-RPC over `grok agent stdio`

duckboard/
  agent.rs, interaction.rs ──── harness dispatch + default resolution
  widget/agent_chat.rs ──────── harness-grouped model picker (UI niceties)
```

### New capabilities

- `harness/grok` — the grok harness over ACP/stdio: maps grok's `session/update` stream
  onto duckchat's neutral `AgentEvent`s (content, reasoning, `tool_call` /
  `tool_call_update`, and `_meta.totalTokens` → usage with the model's `context_window` as
  the denominator), and supports session resume, `bypassPermissions`, and reasoning
  effort.

- `harness/selection` — the explicit provider dimension: a model choice carries its owning
  harness; per-turn sends, worker spawn, and title-summary all dispatch to that harness;
  the default cascade resolves to `grok-4.5`.

- `harness/model-picker` — duckboard UI niceties: models grouped and labeled by harness in
  the picker, the active harness surfaced alongside the observed model, and each model's
  own context window driving the usage meter.

### Modified capabilities

- None. No existing capability spec covers the harness abstraction or model selection; the
  code seams touched (the `Provider` trait, `agent.rs`, `interaction.rs`, `agent_chat.rs`)
  are newly brought under spec by the capabilities above.

### Out of scope

- Removing or deprecating the Claude Code harness — both coexist.
- MCP server configuration for either harness.
- A global (non-project) default-model setting.
- grok-specific features beyond a chat turn (worktrees, best-of-n, memory).
- A per-turn reasoning-effort control in the UI.

## Impact

```
  duckboard picker ──selects──▶ (harness, model) ──▶ TurnRequest{provider, model}
        │                                                     │
        │ grouped by harness                       dispatch on provider
        ▼                                                     ▼
  usage meter uses model.context_window          GrokProvider   ClaudeCodeProvider
                                                 (ACP stdio)     (claude -p)
```

- New `GrokProvider` module plus a persistent-stdio ACP JSON-RPC client — structurally
  different from Claude's per-turn `claude -p` invocation.

- `ModelInfo`, `ModelChoice`, `ChatSession.selected_model`, and `TurnRequest` gain a
  harness identity → a persistence bump in both session files and `config.toml`; existing
  model pins must remain readable.

- Behavior change: the default model becomes `grok-4.5` instead of deferring to the CLI's
  own default.

- Title/summary generation uses `grok-composer-2.5-fast` when the grok harness is active,
  replacing the hardcoded `claude-haiku-4-5`.
