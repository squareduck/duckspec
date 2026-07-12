# @ Chat persistence

## ~ Content blocks

A session message is a role plus an ordered list of content blocks. The kinds are:

```
| Kind                | Role                                              |
| ------------------- | ------------------------------------------------- |
| Text                | Answer (or user/system) prose                     |
| Reasoning           | Assistant thinking, distinct from answer text     |
| ToolUse             | A tool invocation (id, name, input)               |
| ToolResult          | Completion of a prior tool use (same id, output)  |
| UserChoiceQuestion  | Mid-turn structured question text (host display)  |
| UserChoiceAnswer    | Settled pick label or freeform answer (host display) |
```

Reasoning is first-class storage: it round-trips through persist and load as its own kind.
User-choice question and answer blocks are first-class storage as well: they round-trip
through persist and load with their bodies preserved. Session files written before those
kinds existed — containing only Text, Reasoning, ToolUse, and ToolResult — still load.
