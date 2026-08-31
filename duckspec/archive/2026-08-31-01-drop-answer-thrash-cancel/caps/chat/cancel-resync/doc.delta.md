# @ Chat cancel resync

## ~ Why transcripts diverge

Duckboard cancels a turn when the user presses cancel. The transcript keeps answer text
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
