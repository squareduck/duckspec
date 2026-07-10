# @ Chat default prompts

## @ Reply format and order

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

## + Defaults list presentation

When the ready effective list is shown under an empty composer, each suggestion soft-wraps
within the composer width and its row grows with the wrapped lines so the next suggestion
starts below the previous block without overlap. Full suggestion text stays visible (no
ellipsis or hard clip of the displayed value). Empty Enter still sends the full active
string.
