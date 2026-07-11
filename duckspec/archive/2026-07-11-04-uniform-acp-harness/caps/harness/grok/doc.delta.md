# @ Grok harness

The grok harness lets duckboard drive the official grok CLI as a native ACP agent backend.
It supplies the Grok launch and Grok-specific provider behavior (models, attachments,
title model pick); the shared ACP client owns session open/resume, profile event mapping,
and agent-process heat.

## ~ Turn lifecycle

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

## ~ Event translation

grok emits profile `session/update` notifications. The shared ACP client maps them to
neutral agent events (content, reasoning, tool use/result, usage). This harness does not
own a separate client-side mapper; it relies on grok speaking the same dialect profile the
client accepts from every agent.

## + Shared client boundary

Session lifecycle, main-path process reuse, cancel-and-rewarm of the agent child, and
profile event mapping are documented under the ACP client capability. Grok-specific
concerns that remain here:

- native `grok` launch (no intermediate proxy)
- model discovery and context windows from the grok handshake
- title-model fallback
- graceful unavailability when grok cannot launch
- prompt attachment encoding for `session/prompt`
- grok oneshot N=1 isolation on the warm oneshot path
