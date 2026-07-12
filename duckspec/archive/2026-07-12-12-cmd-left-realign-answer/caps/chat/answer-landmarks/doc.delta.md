# @ Chat answer landmarks

## ~ Reply anchors

Previous/next reply navigation steps only across **Answer** segments (the primary reply
body). Thinking and Activity never become jump targets, even when they sit between
Answers.

```
… → Answer_i → (Thinking/Activity) → Answer_{i+1} → …
              ⌘←                  ⌘→
```

Previous (⌘←) is re-align-first:

```
| Viewport vs current Answer top | Previous target         |
| ------------------------------ | ----------------------- |
| Below current top              | Top of current Answer   |
| Already at current top         | Prior Answer (or no-op) |
```

Next (⌘→) always steps to the next Answer when one exists — no re-align-first. At the
first Answer with the viewport already at its top, previous is a no-op; at the last
Answer, next is a no-op. Navigation does not wrap. Stick-to-bottom and mid-message scroll
share the same previous re-align rule.
