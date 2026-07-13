# Stream-tick CPU

Stop open or mid-turn sessions from burning a CPU core: wake the 10 Hz stream UI tick only
when animation or deferred materialize actually needs it, and stop rebuilding the Change
“Changed Files” tree on every iced `view()`.

## Motivation

One Duckboard instance sat at ~70–90% CPU while siblings were idle. Sampling showed the
main thread in continuous iced UI rebuild — Change `view_list` / file-tree flatten and
cosmic_text layout — not the agent child. The 10 Hz `StreamTick` (and 1 Hz flush) stay
subscribed while any session has `is_streaming`, including mid-turn user-choice await.
Each tick rebuilds the whole app view; the Changed Files section rebuilds and re-sorts its
tree from scratch every time.

Why now: the cost is real on a live multi-window setup, and we already agreed A + C as the
scope before inventing recovery or unrelated redraw fixes.

## Intent

- Stream UI tick runs only when at least one session needs stream animation and/or
  stick-to-bottom dirty materialize — not merely because a turn is still open or awaiting
  chips

- Idle awaiting (chips up, agent quiet) does not keep a 10 Hz full-app rebuild pump

- Changed Files list uses a cached tree / flat model updated when the underlying change
  set or expand state changes, not reconstructed on every `view()`

- Idle Duckboard stays near zero CPU when nothing is streaming or animating

## Non-goals

- Detecting or recovering stuck `is_streaming` when `TurnComplete` never arrives
- `pan_row` cursor-move redraw thrash
- File-watcher / gix-status thrash under flush or other disk churn
- Broader view caching outside the Changed Files path
