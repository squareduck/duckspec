# @ Chat default prompts

Under-input **input hints** for the empty composer: an empty session seeds a single entry
from the first lifecycle option when one exists; a non-empty session uses settled agent
oneshot `REPLY:` suggestions only when the global agent input hints setting is enabled
(default off). Empty Enter and Tab cycle that effective list alone.

## ~ Pipeline

Input hints under the empty composer come from one of two sources — never both at once.

**Empty session** (`messages` empty): the effective list is the first lifecycle option in
empty-send form when the session has one (for example `/ds-explore` or `/ds-propose`).
That seed is ready immediately — no model call. If there is no first lifecycle option
(caps, codex, and similar), the list is empty.

**Non-empty session with agent input hints enabled:** after a non-priming agent turn
completes, the session harness runs a short oneshot on its cheapest available model. The
request carries the last assistant message, the preceding user message when present,
discovered slash command names, and the lifecycle heuristic when the session has one — all
as priming context. Message bodies are capped for speed: the assistant tail is at most 40
lines and the user tail at most 12 lines (with a marker when truncated). The model returns
0–3 `REPLY:` lines. The effective list is only that non-empty parse. Fail, timeout, or
empty parse leaves the list empty; the lifecycle heuristic is request context only and
never fills the list.

**Non-empty session with agent input hints disabled:** no oneshot is started; the
effective list stays empty.

```
empty session + lifecycle[0]
    │
    ▼
effective list = [lifecycle[0]]  ──▶  ready (no pending)

non-empty + agent hints OFF
    │
    ▼
no oneshot  ──▶  effective list empty

non-empty + agent hints ON
    │
    ▼
TurnComplete ──▶ cheap-model oneshot
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
become ready (not left pending on the loading indicator). Empty-session disk seed never
uses the loading indicator.

## ~ Reply format and order

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
it, place it in any slot, or invent different replies. For non-empty sessions the
heuristic never populates the effective list. For empty sessions the first lifecycle
option is the list itself (not a soft hint to a model).

## ~ Readiness

```
| State                              | Empty-input chrome     | Empty Enter              |
|------------------------------------|------------------------|--------------------------|
| Main turn streaming                | No list, no loading    | Stream path (queue /     |
|                                    |                        | interrupt) — not send    |
|                                    |                        | active default           |
| Empty session + lifecycle seed     | List (single entry)    | Sends that entry         |
| Pending (oneshot in flight)        | Loading indicator      | No-op (no default sent)  |
| Ready, non-empty list              | Full effective list    | Sends active entry       |
| Ready, empty list                  | No list                | No-op                    |
| Agent hints off, non-empty session | No list                | No-op                    |
| No oneshot outstanding             | Ready (list empty or   | As above                 |
|                                    | populated)             |                          |
| Oneshot fail / timeout settle      | Ready, list empty      | No-op                    |
| Agent handle ended, no settle      | Ready (not pending)    | As ready                 |
```

Superseded oneshot results (generation mismatch) are ignored and do not change the ready
list. While a main turn is streaming, defaults chrome stays hidden even if a settled list
would otherwise be ready.
