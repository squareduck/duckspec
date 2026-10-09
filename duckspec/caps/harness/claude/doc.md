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
heat of that process while the prompt's model and effort still match, catalog
advertisement on initialize, translating Claude's stream into the client's dialect
profile, and bridging AskUserQuestion to the parent's user-choice loop.

## Session ids

Opening a new Claude conversation does not start the official `claude` process. The open
step may use a short-lived ACP handle; the official process starts when the first user
prompt is submitted. Completing that turn surfaces Claude Code's native session id — that
is the id the host persists for resume. A missing session surfaces through the shared
client's session-not-found path.

## Duplex heat

After the first prompt has started Claude, the agent keeps a long-lived `claude` duplex
session (`--input-format stream-json` and `--output-format stream-json`) for the main path
while that process is still the session and was spawned with the prompt's model and
effort. A matching prompt reuses the process. A prompt whose model or effort differs ends
it and starts Claude again with `--resume` for the same native session id, `--model`, and
`--effort` when the prompt has a level. Cancel ends heat; the next turn may start Claude
again and still resume a prior native session id. Changing model or effort during an
in-flight turn waits for the following prompt. Title and reply oneshots do not use this
process.

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

Title summary and reply-suggestion oneshots share the Claude oneshot path. They use the
preferred oneshot model resolved for the Claude harness (global setting or string-match
default) when the agent advertises that model on initialize. If the preferred model is
missing from the advertised list, selection falls back to another advertised model rather
than failing the oneshot. Main chat turns keep the session’s selected model; they are not
forced onto the oneshot preference.

## Model discovery

The host does not fetch the catalog and does not keep a static Claude model table. During
initialize the owned agent fetches the public Claude Code catalog
(`https://downloads.claude.ai/model-catalog/v1/catalog.json`) with no auth and advertises
the rows a reader can select.

```
catalog document
   │  main section, and minimum version absent or met by this claude binary
   ▼
initialize advertise set
   │  name, context window, effort scale when the row has one
   ▼
host model list
```

The binary version is the leading `N.N.N` from `claude --version`, compared
component-wise. When that version cannot be read, only rows with no minimum stay. A row's
effort scale is its levels in catalog order plus the default level, and only when the row
defines one. Initialize also says whether the fetch succeeded, and includes the document's
expiry when the document has one. A failed fetch still finishes initialize and advertises
no models.

The host lists whatever that initialize result advertised. Keeping a prior Claude slice
when a later fetch fails belongs to the harness model catalog.
