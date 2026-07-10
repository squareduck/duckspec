# @ Grok harness

## ~ Turn lifecycle

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

## + Oneshot path

Title summaries and reply suggestions share a separate warm `grok agent stdio` process
from the main conversation path. That process is reused across oneshot calls when hot.
Each oneshot call uses a fresh ACP session (N=1) so prior oneshot prompts do not
accumulate as context; after a call returns, the path opens a new session for the next
oneshot. Oneshot calls use the cheapest available model (with the same fallback as title
selection under Models).
