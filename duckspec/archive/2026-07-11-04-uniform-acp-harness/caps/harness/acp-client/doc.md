# ACP client

Shared Agent Client Protocol (ACP) client runtime for every coding harness. Providers only
supply the agent launch and harness-specific framing; the client owns spawn, session
open/resume, event mapping, and main-path process heat.

## Role

```
Provider (grok | claude-code | …)
   │  AgentLaunch + turn request
   ▼
ACP client runtime
   │  JSON-RPC over stdio
   ▼
Agent process (native or workspace adapter)
```

Duckboard and the chat worker never speak a harness-specific wire protocol for turns.
Switching harness changes which launch the provider builds, not which client stack runs
the turn.

## Turn lifecycle

The main path keeps one agent child warm across turns when possible:

```
[if cold] spawn launch + initialize
   │
session/new        (no prior session id)   ─┐
session/load       (prior session id)      ─┴─▶ session id (surfaced to caller)
   │
session/prompt                 stream session/update → agent events
   │                           (agent may rebind session id → surface again)
   │
process stays up               (until cancel or handle shutdown)
```

Cancel kills the main agent child. The next turn may spawn again and still resume a
conversation session id when one is supplied.

Agent→client requests during a turn (for example permission prompts) are auto-answered so
a headless host never deadlocks waiting on UI.

## Dialect profile

The client accepts a fixed profile of `session/update` shapes and maps them to neutral
agent events. Adapters that are not first-party ACP agents must emit this profile; native
agents (such as grok) already do.

```
session/update                 agent event
─────────────────────────────  ─────────────────────────────
agent_message_chunk        →   content (assistant text)
agent_thought_chunk        →   reasoning (separate channel)
tool_call                  →   tool use  (id, name, input)
tool_call_update (done)    →   tool result (same id, output)
total-token telemetry      →   usage update (used + window)
```

Missing optional shapes (for example reasoning chunks) simply produce no reasoning events;
they do not fail the turn.

## Session identity

Session ids are agent-assigned and harness-bound. The client surfaces the id returned at
open/load for persistence, and if the agent rebinds the id during the turn (a different id
than open returned), surfaces the rebound id so the host persists the durable one for the
next `session/load`. A load that fails because the session is gone surfaces
session-not-found so the host can clear the id and open a fresh session.

## Oneshot path

Title summaries and reply suggestions use the same ACP client machinery on a separate
oneshot path (process heat and N=1 isolation are owned by the warm runtime capability).
The client does not resume a prior oneshot conversation across those calls.
