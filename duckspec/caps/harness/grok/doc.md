# Grok harness

The grok harness lets duckboard drive the grok CLI as an agent backend. It implements the
same provider contract as the Claude harness, so a chat turn, model list, and title
summary all flow through one shared interface regardless of which backend runs them.

## Turn lifecycle

The main path keeps a `grok agent stdio` process warm across turns when possible and
speaks ACP (the Agent Client Protocol, JSON-RPC 2.0 over stdio). The first turn (or the
first turn after a cancel) spawns the child and runs `initialize`. Each turn then opens a
logical session and prompts:

```
[if cold] spawn + initialize
   │
session/new        (no prior session id)   ─┐
session/load       (prior session id)      ─┴─▶ obtain the session id
   │
session/prompt                 send the user's message, stream the reply
   │
process stays up               (until cancel or handle shutdown)
```

A turn started without a session id opens a new session; a turn started with one resumes
it, so earlier conversation context is available to the model. The session id grok reports
is surfaced back to the caller to persist and reuse on the next turn. Because the id
identifies a grok-side conversation, it is only meaningful to the grok harness — it cannot
be resumed by another backend.

Cancel kills the main child. The next turn may spawn again and still resume the
conversation session id when one is supplied.

Tool execution is auto-approved for the turn; the harness answers any permission request
the agent raises so a turn never stalls waiting on input.

## Event translation

grok streams `session/update` notifications during a turn. The harness maps each to a
neutral agent event so the rest of duckboard renders grok and Claude turns identically:

```
grok session/update            duckboard agent event
─────────────────────────────  ─────────────────────────────
agent_message_chunk        →   content (assistant text)
agent_thought_chunk        →   reasoning (separate channel)
tool_call                  →   tool use  (id, name, input)
tool_call_update (done)    →   tool result (same id, output)
total-token telemetry      →   usage update (used + window)
assigned session id        →   session id update
prompt completed           →   turn complete
error / abnormal stop      →   error
```

Assistant text and reasoning travel on distinct channels, so thinking can be shown apart
from the answer. A tool invocation always surfaces as a use event followed by a result
event that shares the same call id, letting a caller pair them.

## Context usage

Every token-telemetry update carries the running used-token count together with the active
model's context window, so the usage meter reflects true context fill rather than an
estimate. The context window is taken from the model's own advertised size, not inferred
from the stream.

## Models

The harness discovers grok's models from the ACP handshake, which advertises the available
models and each model's context window. Every model it returns is tagged with the grok
harness so it stays distinguishable once merged with other backends' models.

Title summaries — the short session names generated after the first reply — use the
cheapest available model. When the preferred fast model is not available on the account,
the harness falls back to another available model instead of failing.

## Unavailability

The grok backend is optional. When the grok binary cannot be launched or is not
authenticated, model discovery returns an empty list and running a turn fails with a typed
error. The harness never panics on a missing or unauthenticated backend, so duckboard
simply behaves as though grok is not offered.

## Prompt attachments

A turn request may carry binary attachments keyed by id, with the prompt text referring to
them as markdown links of the form `[label](attach:<id>)`. Before sending
`session/prompt`, the harness walks those markers and builds a multi-block ACP content
array instead of a single unexpanded text string.

```
prompt text + attachments map
        │
        ▼
  walk [label](attach:<id>)
        │
        ├── resolved image/*  →  ACP image block (mimeType + base64 data)
        ├── resolved other    →  text block naming the attachment
        └── unresolved id     →  original markdown left as text
        │
        ▼
  session/prompt.prompt: [ text | image | text | … ]
```

System-prompt additions still fold into the leading text (blank-line separated) ahead of
the user message; attach markers normally appear only in that user message text.
Selection-context chips are separate: they are plain text prepended by the caller and do
not use the attachments map.

## Oneshot path

Title summaries and reply suggestions share a separate warm `grok agent stdio` process
from the main conversation path. That process is reused across oneshot calls when hot.
Each oneshot call uses a fresh ACP session (N=1) so prior oneshot prompts do not
accumulate as context; after a call returns, the path opens a new session for the next
oneshot. Oneshot calls use the cheapest available model (with the same fallback as title
selection under Models).
