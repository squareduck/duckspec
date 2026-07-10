# Chat default prompts

Conversation-local empty-input defaults from a cheap-model oneshot: parse ordered `REPLY:`
suggestions (heuristic passed only as a soft hint), show and arm them only after the
oneshot settles, and drive empty Enter plus Tab cycling from that list alone.

## Pipeline

After a non-priming agent turn completes, the session harness runs a short oneshot on its
cheapest available model. The request carries the last assistant message, the preceding
user message when present, discovered slash command names, and the lifecycle heuristic
when the session has one — all as priming context. The model returns 0–3 `REPLY:` lines;
that parse is the entire effective list. Nothing is appended afterward.

```
TurnComplete
    │
    ▼
cheap-model oneshot  (assistant, user?, commands, heuristic?)
    │
    ▼
REPLY: lines (0–3)  ──▶  effective list (parse only)
    │
    ▼
pending → ready  ──▶  empty composer: list + active entry
```

While a oneshot is outstanding, empty-input chrome shows a loading indicator instead of a
list. Starting a new turn invalidates any in-flight oneshot so a late result cannot arm
stale defaults.

## Reply format and order

The oneshot must answer only with lines of the form `REPLY: <text>`. Parsing keeps those
lines in order, trims the text, drops empties, and hard-caps at three. Other lines are
ignored. Slash forms the model invents are kept as written.

When multiple lines are emitted, the instruction asks for this order:

```
| Position | Role                                      |
|----------|-------------------------------------------|
| first    | Most obvious continue for the flow        |
| middle   | Alternatives                              |
| last     | Negative / decline when that fits         |
```

The lifecycle heuristic is a soft hint only. The model may omit it, place it in any slot,
or invent different replies.

## Readiness

```
| State                         | Empty-input chrome     | Empty Enter              |
|-------------------------------|------------------------|--------------------------|
| Pending (oneshot in flight)   | Loading indicator      | No-op (no default sent)  |
| Ready, non-empty list         | Full effective list    | Sends active entry       |
| Ready, empty list             | No list                | No-op                    |
| No oneshot outstanding        | Ready (list may empty) | As above                 |
```

Superseded oneshot results (generation mismatch) are ignored and do not change the ready
list.

## Empty composer

When the input is empty and suggestions are ready, the effective list is the set of
one-Enter defaults:

- **Enter** sends the active entry (index into the list).

- **Tab** / **Shift-Tab** move the active index with wrap, without filling the input.

- The active entry is the primary row; other options are shown fainter so the current
  default stays obvious.

- Slash-command completion still owns Tab when its popup is visible.

When the input is non-empty, normal typing and send apply; the default list is not used.
While pending, empty Enter does not send a default; typed non-empty send is unchanged.
