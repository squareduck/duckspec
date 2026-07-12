# Chat answer landmarks

Full-width contrast on the latest Answer and keyboard jumps between Answer starts and chat
history ends, so long transcripts stay easy to re-enter.

## Last Answer band

User prompts already read as cards. Answers stay plain prose on the chat surface — except
the **latest non-empty Answer**, which gets a full-width, slightly more contrasty
background across the chat column. The band is a surface lift only: no bubble border, no
card-style side inset.

```
| Segment              | Band?                                      |
| -------------------- | ------------------------------------------ |
| Latest non-empty Answer | yes — full-width contrast               |
| Older Answers        | no — ordinary chat surface                 |
| Empty Answer (no body) | no — band waits until Answer text exists |
| Thinking / Activity  | no                                         |
| User / System        | no (User keeps its existing card)          |
```

The target always tracks the latest non-empty Answer in transcript order, including while
a turn is still streaming once Answer text is present. Jumping to an older Answer does not
move the band — navigation and highlight are independent.

## Reply anchors

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

## Current Answer for jumps

Which Answer is “current” for previous/next depends on the viewport:

```
| Situation                         | Current Answer                              |
| --------------------------------- | ------------------------------------------- |
| Stuck to bottom                   | Last Answer anchor                          |
| Scrolled into history             | Last Answer whose top is at or above the viewport top |
| No Answer above the viewport top  | First Answer anchor                         |
| No Answers                        | None — previous/next no-op                  |
```

## History ends

```
| Shortcut intent | Effect                                              |
| --------------- | --------------------------------------------------- |
| History top     | Viewport to the start of the transcript; unstick    |
| History bottom  | Viewport to the end; stick so streaming follows end |
```

Leave-bottom jumps (history top, previous/next Answer) clear stick-to-bottom so the
viewport does not fight the user.

## Shortcuts

Landmark shortcuts apply when the chat interaction is active (session present), including
while the composer is focused. Command-modified arrows move the transcript; bare arrows
stay with the composer caret. Modals that already own navigation keys keep that ownership
while open.
