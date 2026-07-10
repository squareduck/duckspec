# Chat default prompts

Conversation-local empty-input defaults from a cheap-model oneshot: parse ordered `REPLY:`
suggestions (lifecycle heuristic passed only as a soft request hint), show and arm them
only after a non-empty parse settles, and drive empty Enter plus Tab cycling from that
list alone. The effective list is never seeded or filled from the lifecycle heuristic.

## Pipeline

After a non-priming agent turn completes, the session harness runs a short oneshot on its
cheapest available model. The request carries the last assistant message, the preceding
user message when present, discovered slash command names, and the lifecycle heuristic
when the session has one — all as priming context. Message bodies are capped for speed:
the assistant tail is at most 40 lines and the user tail at most 12 lines (with a marker
when truncated). The model returns 0–3 `REPLY:` lines.

The effective list is only a non-empty oneshot parse. Before any oneshot has produced a
non-empty parse — and again when an oneshot fails, times out, or returns nothing — the
list is empty. The lifecycle heuristic is never an effective-list entry; it is request
context only.

```
new session / no useful oneshot yet
    │
    ▼
effective list empty  ──▶  ready (no composer defaults)

TurnComplete
    │
    ▼
cheap-model oneshot  (assistant, user?, commands, heuristic?)
    │
    ├── REPLY: lines (1–3)  ──▶  effective list (parse only)
    └── fail / timeout / empty  ──▶  effective list empty
    │
    ▼
pending → ready  ──▶  empty composer: list when non-empty
```

While a oneshot is outstanding, empty-input chrome shows a loading indicator instead of a
list. Starting a new turn invalidates any in-flight oneshot so a late result cannot arm
stale defaults. If the chat agent ends while a oneshot is still outstanding, suggestions
become ready (not left pending on the loading indicator).

## Reply format and order

The oneshot must answer only with lines of the form `REPLY: <text>`. Parsing keeps those
lines in order, trims the text, drops empties, and hard-caps at three. Other lines are
ignored. Slash forms the model invents are kept as written. The shared instruction
soft-asks that each REPLY text be at most 100 characters; the parser does not enforce that
budget — longer replies stay in the list in full.

When multiple lines are emitted, the instruction asks for this order:

```
| Position | Role                                      |
|----------|-------------------------------------------|
| first    | Most obvious continue for the flow        |
| middle   | Alternatives                              |
| last     | Negative / decline when that fits         |
```

On the oneshot request, the lifecycle heuristic is a soft hint only — the model may omit
it, place it in any slot, or invent different replies. The heuristic never populates the
effective list.

## Readiness

```
| State                         | Empty-input chrome     | Empty Enter              |
|-------------------------------|------------------------|--------------------------|
| Main turn streaming           | No list, no loading    | Stream path (queue /     |
|                               |                        | interrupt) — not send    |
|                               |                        | active default           |
| Pending (oneshot in flight)   | Loading indicator      | No-op (no default sent)  |
| Ready, non-empty list         | Full effective list    | Sends active entry       |
| Ready, empty list             | No list                | No-op                    |
| No oneshot outstanding        | Ready (list empty or   | As above                 |
|                               | parse-only)            |                          |
| Oneshot fail / timeout settle | Ready, list empty      | No-op                    |
| Agent handle ended, no settle | Ready (not pending)    | As ready                 |
```

Superseded oneshot results (generation mismatch) are ignored and do not change the ready
list. A new session starts ready with an empty list until a non-empty oneshot parse
settles. While a main turn is streaming, defaults chrome stays hidden even if a settled
oneshot list would otherwise be ready.

## Defaults list presentation

When the ready effective list is shown under an empty composer, each suggestion soft-wraps
within the composer width and its row grows with the wrapped lines so the next suggestion
starts below the previous block without overlap. Full suggestion text stays visible (no
ellipsis or hard clip of the displayed value). Empty Enter still sends the full active
string.
