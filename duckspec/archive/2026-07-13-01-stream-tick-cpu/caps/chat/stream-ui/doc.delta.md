# @ Chat stream UI

## @ When materialization runs

```
| Trigger                              | Materialize?                                |
| ------------------------------------ | ------------------------------------------- |
| Answer / reasoning content only      | No — mark dirty; wait for tick              |
| Stream UI tick (dirty + stick)       | Yes — fold in accumulated pure text         |
| Stream UI tick (dirty, scrolled up)  | No — leave dirty; keep history scroll calm  |
| Re-stick to bottom while dirty       | Yes — paint deferred pure content           |
| Tool use / tool result               | Yes — immediately (even if scrolled up)     |
| Answer ↔ reasoning channel switch    | Yes — immediately (draft need not commit)   |
| Turn complete / error / exit         | Yes — immediately                           |
| Load / send / non-stream paths       | Yes — immediately                           |
```

Pure content deltas can arrive many times per second. The stream UI tick (~100 ms) drains
dirtiness when the user is following the live answer (stick-to-bottom) — but only while a
session **needs** that tick. Need is true while the agent is working (streaming and not
awaiting a user choice), and also when deferred pure-content materialize is owed on
stick-to-bottom even if chips are up. Idle mid-turn await with nothing to paint does not
keep the tick running, so the app is not rebuilt at 10 Hz while the user thinks.

If they have scrolled up to read history, pure-content dirtiness stays deferred so the
chat column is not rebuilt under their scroll; returning to the bottom paints the deferred
text. Structural events change transcript shape (new Activity, open Thinking under a live
draft, final Answer) and must paint in the same turn as the session update.
