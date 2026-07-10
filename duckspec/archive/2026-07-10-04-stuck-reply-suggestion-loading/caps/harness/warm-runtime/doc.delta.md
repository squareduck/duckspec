# @ Warm agent runtime

## ~ Paths on a handle

```
  AgentHandle (one chat)
  │
  ├─ main path
  │     conversation turns
  │     process-hot after first send when the harness supports it
  │     cancel ends main heat; next turn may re-warm
  │
  └─ oneshot path (single, shared)
        title summary + reply suggestions
        one request at a time
        fresh logical session every call (N=1)
        each call budgeted (10s wall-clock); over-budget fails to the caller
        any failure or timeout cold-resets oneshot heat before the next call
```

Main and oneshot are independent: cancelling a turn does not require tearing down the
oneshot path. Title and reply work never share the main conversation session.

## ~ Oneshot isolation (N=1)

Each title or reply-suggestion call is a one-shot against a fresh logical session. After
the result returns, the next oneshot call does not resume the previous oneshot
conversation, so earlier suggestion prompts do not accumulate as context. Isolation is
about logical session, not necessarily killing a process: a harness may keep a child warm
and open a new session between successful calls.

Each oneshot call is also wall-clock bounded: ensure-hot plus prompt for that call must
finish within ten seconds or the call fails to the caller. Title and reply each get a full
ten-second budget; they still serialize on the shared path. After any oneshot failure —
including timeout — oneshot process heat is cold-reset so a later oneshot on the same
handle can run without waiting on a wedged prior call.
