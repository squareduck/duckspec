# @ Chat persistence

## ~ In-flight turn durability

A turn streams messages incrementally. If those messages live only in memory until the
turn completes, anything that disturbs the scope's state before completion loses them. Two
rules close that window.

- **Flush before mutate.** Any operation that migrates, replaces, or drops a scope's
  in-memory state first persists that scope's sessions. An in-flight turn is therefore
  written to disk before, for example, a promotion moves the scope.

- **Eager persistence.** While a turn streams, its session is persisted periodically — not
  only at turn completion — so an abrupt termination loses at most a bounded tail of the
  turn rather than the whole turn. Writes are coalesced so a long turn does not rewrite
  the growing session file on every message. In-flight answer text and in-flight reasoning
  are both folded into the snapshot: pending answer text as Text content, pending
  reasoning as Reasoning content (never as Text).

Together with turn-boundary saves, these bound the loss window to a short tail in the
worst case and to nothing in the ordinary migrate/promote paths.

## + Content blocks

A session message is a role plus an ordered list of content blocks. The kinds are:

```
| Kind       | Role                                              |
| ---------- | ------------------------------------------------- |
| Text       | Answer (or user/system) prose                     |
| Reasoning  | Assistant thinking, distinct from answer text     |
| ToolUse    | A tool invocation (id, name, input)               |
| ToolResult | Completion of a prior tool use (same id, output)  |
```

Reasoning is first-class storage: it round-trips through persist and load as its own kind.
Session files written before Reasoning existed — containing only Text, ToolUse, and
ToolResult — still load.
