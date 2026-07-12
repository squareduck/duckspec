# Chat cancel resync

Keeps the agent's own conversation history in agreement with the transcript the user sees
after a turn is cancelled, by carrying the cancelled turn's kept answer draft into the
next send.

## Why transcripts diverge

Duckboard cancels a turn in two situations: the user presses cancel, or the answer-thrash
budget trips and the last draft is kept. In both cases the transcript keeps answer text
the user can read and respond to — but the agent runtime never records a reply that was
still streaming when the turn was cancelled. The user's next message then answers text the
agent has no memory of sending. A bare token like `confirm` lands on the wrong gate, and
the agent re-presents work the user already accepted.

```
duckboard transcript            agent runtime history
────────────────────            ─────────────────────
reply draft (kept)              (nothing — turn cancelled)
user: confirm          ──────►  confirm … of what?
```

## Capture rule

Only the **uncommitted in-flight draft** at cancellation is captured. Answer text
committed at a tool boundary is already recorded in the agent runtime's own history, so
committing it again would duplicate context. When the in-flight draft is empty at
cancellation, both sides already agree and nothing is recorded.

The captured draft is stored on the session and persists with it, so the resync survives
an app restart between the cancellation and the next send.

## Resync reminder

The next prompt sent on the session carries the captured draft **after** the user's text,
framed as the user-visible reply of an interrupted turn that the agent should treat as its
own already-sent message. Placing it after the user's text keeps slash commands at the
start of the prompt. The reminder rides exactly one send; after that the session holds no
unsynced draft until another cancellation captures one.
