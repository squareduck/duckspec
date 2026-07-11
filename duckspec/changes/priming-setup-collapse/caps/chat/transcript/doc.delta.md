# @ Chat transcript

## ~ Collapse defaults

```
| Segment        | Live default | Settled / reload default | Auto-collapse                     |
| -------------- | ------------ | ------------------------ | --------------------------------- |
| Thinking       | expanded     | collapsed                | when Answer follows or turn ends  |
| Activity       | expanded     | collapsed                | when Answer follows or turn ends  |
| Answer         | always shown | always shown             | not collapsible                   |
| User (normal)  | always shown | always shown             | not collapsible                   |
| User (priming) | collapsed    | collapsed                | timed re-hide after expand (~15s) |
```

If the user toggles a Thinking or Activity segment, that choice wins: later auto-collapse
does not force it shut again for that segment.

The synthetic first-turn priming user message (AGENTS.md / orientation inject, answered
with `.`) is presented as a collapsible **Setup** block. It starts collapsed so
scroll-to-top reaches the first real user message. Click expands it; after a short delay
it auto-collapses again. Sync rebuilds do not force-shut a user-expanded Setup block —
only the expand timer (or a manual collapse) does.

Collapsed Thinking labels use line count (`Thinking · N lines`), not duration. Collapsed
priming Setup labels use the same line-count pattern (`Setup · N lines`).
