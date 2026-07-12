# @ ACP client

## ~ Turn lifecycle

The main path keeps one agent child warm across turns when possible:

```
[if cold] spawn launch + initialize
   │
session/new        (no prior session id)   ─┐
session/load       (prior session id)      ─┴─▶ session id (surfaced to caller)
   │
session/prompt                 stream session/update → agent events
   │                           mid-turn agent→client requests (below)
   │                           (agent may rebind session id → surface again)
   │
process stays up               (until cancel or handle shutdown)
```

Cancel kills the main agent child. The next turn may spawn again and still resume a
conversation session id when one is supplied.

Mid-turn agent→client requests on the main path:

```
| Request                                                | Main path                                      |
|--------------------------------------------------------|------------------------------------------------|
| `session/request_permission` (allow/reject kinds only) | Auto-select allow; no host UI                  |
| Permission product options (not only allow/reject)     | User-choice event; prompt from toolCall title when present |
| Structured question (`x.ai/ask_user_question`, etc.)   | User-choice event; prompt from questionnaire; park for host |
| Unknown method                                         | Safe non-blocking completion                   |
```

The host answers a parked choice with a selection, a custom freeform answer, or cancel;
turn cancel also completes a pending choice as cancelled. Custom freeform completes the
request successfully (answer payload is the free text), not as cancelled. Oneshot path
never parks on host UI for these requests.
