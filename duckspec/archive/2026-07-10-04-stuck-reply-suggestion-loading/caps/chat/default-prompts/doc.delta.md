# @ Chat default prompts

## ~ Pipeline

After a non-priming agent turn completes, the session harness runs a short oneshot on its
cheapest available model. The request carries the last assistant message, the preceding
user message when present, discovered slash command names, and the lifecycle heuristic
when the session has one — all as priming context. Message bodies are capped for speed:
the assistant tail is at most 40 lines and the user tail at most 12 lines (with a marker
when truncated). The model returns 0–3 `REPLY:` lines.

Before any oneshot has produced a non-empty parse — and again when an oneshot fails, times
out, or returns nothing — the effective list is the lifecycle heuristic alone (whatever
the phase ladder set: `ds-explore`, `ds-spec`, …), formatted for empty send. A non-empty
oneshot parse replaces that list entirely; the heuristic is not merged in.

```
new session / no useful oneshot yet
    │
    ▼
lifecycle heuristic  ──▶  effective list (0–1 entry)  ──▶  ready immediately

TurnComplete
    │
    ▼
cheap-model oneshot  (assistant, user?, commands, heuristic?)
    │
    ├── REPLY: lines (1–3)  ──▶  effective list (parse only)
    └── fail / timeout / empty  ──▶  heuristic fallback (if present)
    │
    ▼
pending → ready  ──▶  empty composer: list + active entry
```

While a oneshot is outstanding, empty-input chrome shows a loading indicator instead of a
list. Starting a new turn invalidates any in-flight oneshot so a late result cannot arm
stale defaults. If the chat agent ends while a oneshot is still outstanding, suggestions
become ready (not left pending on the loading indicator).

## ~ Readiness

```
| State                         | Empty-input chrome     | Empty Enter              |
|-------------------------------|------------------------|--------------------------|
| Main turn streaming           | No list, no loading    | Stream path (queue /     |
|                               |                        | interrupt) — not send    |
|                               |                        | active default           |
| Pending (oneshot in flight)   | Loading indicator      | No-op (no default sent)  |
| Ready, non-empty list         | Full effective list    | Sends active entry       |
| Ready, empty list             | No list                | No-op                    |
| No oneshot outstanding        | Ready (list may be     | As above                 |
|                               | heuristic or empty)    |                          |
| Oneshot fail / timeout settle | Ready (heuristic if    | As ready                 |
|                               | present)               |                          |
| Agent handle ended, no settle | Ready (not pending)    | As ready                 |
```

Superseded oneshot results (generation mismatch) are ignored and do not change the ready
list. A new session with a heuristic is ready with that single default without waiting for
a model. While a main turn is streaming, defaults chrome stays hidden even if a heuristic
or settled oneshot list would otherwise be ready.
