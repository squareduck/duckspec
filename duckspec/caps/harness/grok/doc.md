# Grok harness

The grok harness lets duckboard drive the grok CLI as an agent backend. It implements the
same provider contract as the Claude harness, so a chat turn, model list, and title
summary all flow through one shared interface regardless of which backend runs them.

The grok harness lets duckboard drive the official grok CLI as a native ACP agent backend.
It supplies the Grok launch and Grok-specific provider behavior (models, attachments,
title model pick); the shared ACP client owns session open/resume, profile event mapping,
and agent-process heat.

## Turn lifecycle

The main path uses the shared ACP client with a Grok launch (`grok` agent stdio, including
always-approve / no-ask-user flags as required for headless host use). The client keeps
that agent process warm across turns when possible, opens or resumes a grok session, and
prompts:

```
[if cold] spawn grok launch + initialize
   │
session/new | session/load     (shared client)
   │
session/prompt                 stream profile updates → agent events
   │
process stays up               (until cancel or handle shutdown)
```

The session id grok assigns is harness-bound: it cannot be resumed by another backend.
Cancel kills the main agent child; the next turn may spawn again and still resume that id
when supplied.

Tool execution is auto-approved for the turn; permission requests from the agent are
auto-answered so a turn never stalls waiting on host UI.

## Event translation

grok emits profile `session/update` notifications. The shared ACP client maps them to
neutral agent events (content, reasoning, tool use/result, usage). This harness does not
own a separate client-side mapper; it relies on grok speaking the same dialect profile the
client accepts from every agent.

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

## Shared client boundary

Session lifecycle, main-path process reuse, cancel-and-rewarm of the agent child, and
profile event mapping are documented under the ACP client capability. Grok-specific
concerns that remain here:

- native `grok` launch (no intermediate proxy)
- model discovery and context windows from the grok handshake
- title-model fallback
- graceful unavailability when grok cannot launch
- prompt attachment encoding for `session/prompt`
- grok oneshot N=1 isolation on the warm oneshot path
