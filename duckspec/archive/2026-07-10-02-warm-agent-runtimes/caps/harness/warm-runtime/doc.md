# Warm agent runtime

Per-chat agent handle owns a main path and a single oneshot path for turns, title
summaries, and reply suggestions. Process heat is lazy and harness-specific; callers
always go through the handle.

## Paths on a handle

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
```

Main and oneshot are independent: cancelling a turn does not require tearing down the
oneshot path. Title and reply work never share the main conversation session.

## Lazy activation

Neither path must be hot when the handle becomes ready. The first turn activates the main
path as needed. Oneshot work after the first send does not require a separate pre-warm API
from the UI — the handle activates the oneshot path when first used (and may warm it in
the background when the first send starts).

## Oneshot isolation (N=1)

Each title or reply-suggestion call is a one-shot against a fresh logical session. After
the result returns, the next oneshot call does not resume the previous oneshot
conversation, so earlier suggestion prompts do not accumulate as context. Isolation is
about logical session, not necessarily killing a process: a harness may keep a child warm
and open a new session between calls.

## Cold-capable harnesses

Some harnesses cannot (or do not yet) keep a long-lived process. They still implement the
same handle API and perform equivalent work per call. Callers do not branch on heat
capability when requesting titles or reply suggestions.

## Relationship to other capabilities

- **Harness selection** chooses which provider backs the handle for a chat.

- **Grok harness** (when selected) reuses real agent processes on these paths.

- **Chat default prompts** still owns suggestion parse, readiness, and empty-input chrome;
  this capability only owns how those oneshots are executed through the handle.
