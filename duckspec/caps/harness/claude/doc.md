# Claude harness

Duckboard drives Claude Code through the shared ACP client and an owned workspace agent
binary. The agent wraps the official `claude` CLI; the host never speaks Claude's
stream-json protocol itself and never depends on npm or Node for the Claude path.

## Process tree

```
duckboard / duckchat worker
   │  ACP (shared client) — mid-turn user choice when Claude asks
   ▼
duckchat-claude-acp          (owned agent)
   │  stream-json duplex + control / canUseTool
   ▼
claude                       (official CLI)
```

Selecting the Claude harness only changes the provider launch (the agent binary). Turn
lifecycle, event mapping, and main heat for the **agent** process are the shared ACP
client. This capability owns Claude-specific behavior: agent binary discovery, when the
inner `claude` process starts, Claude-native session ids after the first prompt, duplex
heat of that process, translating Claude's stream into the client's dialect profile, and
bridging AskUserQuestion to the parent's user-choice loop.

## Session ids

Opening a new Claude conversation does not start the official `claude` process. The open
step may use a short-lived ACP handle; the official process starts when the first user
prompt is submitted. Completing that turn surfaces Claude Code's native session id — that
is the id the host persists for resume. A missing session surfaces through the shared
client's session-not-found path.

## Duplex heat

After the first prompt has started Claude, the agent keeps a long-lived `claude` duplex
session (`--input-format stream-json` and `--output-format stream-json`) for the main path
when possible. A second main turn reuses that process. Cancel ends heat; the next turn may
start Claude again and still resume a prior native session id.

## Profile emission

The agent emits the shared client's dialect profile so one client mapper serves Claude and
Grok:

```
Claude stream                 profile session/update
────────────────────────────  ─────────────────────────────
assistant text deltas     →   agent_message_chunk
thinking deltas           →   agent_thought_chunk
tool use / result         →   tool_call / tool_call_update
```

Updates are delivered to the host as they arrive from Claude during the turn, not held
until the prompt result. When Claude does not produce thinking, no thought chunks are
emitted.

## Agent binary discovery

```
1. DUCKCHAT_CLAUDE_ACP (explicit override)
2. sibling of the running executable
3. PATH
```

Local builds place `duckchat-claude-acp` next to `duckboard` under `target/`. If no binary
can be launched, a Claude turn fails with a typed error — the same operator class as a
missing Grok binary.

## Backend boundary

The agent translates protocols only. Tool execution, auth, skills, and Claude Code
behavior stay inside the official `claude` CLI. The harness does not reimplement Claude
over the Messages API and does not use community npm ACP adapters.

## Structured questions

Claude may call `AskUserQuestion` during a turn. The owned agent is not configured to
disallow that tool. When Claude asks, the agent maps the control / canUseTool request to a
parent ACP choice so the host can show options. A host selection completes as allow with
`updatedInput` carrying the original `questions` and an `answers` map (question text →
selected option label). A host custom freeform answer completes the same way with free
text as the answer value (not deny). Host cancel finishes without accepting the
questionnaire.

Ordinary tools stay on permission bypass: they do not open the host choice UI. Only
structured clarifying questions use the mid-prompt parent choice path.

## Oneshot preferred model

Title summary and reply-suggestion oneshots share the Claude oneshot path and prefer the
curated cheap/fast model alias (`haiku`) when the agent advertises it on initialize. If
that alias is missing from the advertised list, selection falls back to another advertised
model rather than failing the oneshot. Main chat turns keep the session’s selected model;
they are not forced onto the oneshot preference.
