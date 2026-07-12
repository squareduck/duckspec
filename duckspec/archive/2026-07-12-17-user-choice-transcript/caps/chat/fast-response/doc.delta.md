# @ Chat fast response

Source-neutral option chips for mid-turn structured choices and settled oneshot reply
suggestions: ordered options with ⌘-number activation, ephemeral view layout, and
empty-send formatting for bare skill names. A live user-choice request fills the shell for
in-band answers and shows the question as a chip above the options when prompt text is
present. Settling commits host question and answer transcript chips; cancel commits
neither. Freeform submit while awaiting is a custom answer to the pending question.

## ~ Shell model

Fast response is a thin option shell with an activation source. User-choice fills may also
carry question text for live display and for the settled host log.

```
| Field   | Role                                                          |
|---------|---------------------------------------------------------------|
| options | Ordered choices; ⌘1…⌘n when chips are visible                 |
| source  | Why the shell is filled — drives activation                   |
| prompt  | Question text on user-choice fills (optional)                 |
```

```
| Source        | Filled by                         | Activation                         |
|---------------|-----------------------------------|------------------------------------|
| User choice   | Mid-turn structured question      | In-band answer (not a new agent turn) |
| Oneshot hints | Settled freeform reply suggestions| Normal user message send           |
| (empty)       | Nothing                           | No-op                              |
```

There is no cancel chip and no ⌘⌫ binding on the shell. Turn cancel (esc esc) completes a
parked choice as cancelled on the agent wire and does not leave host question/answer
entries. Composer submit while awaiting completes it as a custom freeform answer.

Live option chips are view chrome until settle. On settle (pick or freeform), the host
session stores a question chip entry when prompt text was present and an answer chip entry
for the chosen label or freeform text (no hotkey on the stored answer). Unselected options
are not stored. Oneshot-hint activation still sends the option text as a normal user turn.
Empty-send formatting (bare `ds-foo` → `/ds-foo`) remains available for other
empty-composer bootstrap consumers; it does not imply the shell is filled from disk phase.

## @ Freeform while awaiting (custom answer)

When chips reflect a live user choice and the user types freeform text then submits:

```
awaiting choice + non-empty submit
        │
        ├─ complete pending choice as custom answer (freeform text)
        ├─ commit host Q (if any) + answer chips
        ├─ clear option shell
        └─ harness maps freeform into the question answer value
           (not cancel/skip + next user turn; not interrupt-queue only)
```

```
| Input            | Meaning        | Choice completion      | Host log              |
|------------------|----------------|------------------------|-----------------------|
| ⌘n chip          | structured pick| selected option        | Q (if any) + label    |
| Composer Enter   | custom answer  | freeform text as answer| Q (if any) + freeform |
| Esc esc          | dismiss        | cancelled              | nothing               |
```

## + Live question chip

While awaiting a user choice with non-empty question text, a question chip sits above the
numbered option chips in the scroll column. The label uses the form `Question: <text>`
(prefix added when not already present). It uses the same chip geometry as options but
chat-area fill (not the quiet-accent option treatment) and is not a selectable ⌘ option.
Missing or empty question text omits that chip; options still follow ordinary visibility.

## + Settled choice transcript

```
awaiting choice
        │
   ┌────┴────────────────┐
   │ pick / freeform     │ cancel
   ▼                     ▼
host: Q chip (if any)    host: nothing
      + answer chip
wire: in-band answer     wire: cancelled
```

```
| Entry    | When stored                         | Label content                         |
|----------|-------------------------------------|---------------------------------------|
| Question | Settle with non-empty prompt        | `Question: <text>` (prefix if needed) |
| Answer   | Settle with pick or freeform        | Option label or freeform, no hotkey   |
| (none)   | Cancel                              | —                                     |
```

Settled chips render with the same chip language as the live prompt so reloaded sessions
match what the user saw when answering.
