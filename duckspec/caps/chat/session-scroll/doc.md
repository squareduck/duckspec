# Chat session scroll

When the active chat session is opened or switched, the transcript shows the latest
content; when only the area changes, that session’s remembered viewport is restored.
Layout reflow preserves scroll only while session identity is unchanged.

## Session identity

The active chat is identified by scope plus session id. A different scope or a different
session tab under the same scope is a new identity. Re-selecting the same already-active
session is not.

## Viewport policy

```
| Trigger | Viewport |
| --- | --- |
| Intentional open or switch (scope pick, session tab, new/clear session, open change/exploration) | Latest content; stick-to-bottom engaged |
| Area navigation only (same session identity when returning) | Restored memory: stick-to-bottom or last offset |
| Layout reflow with unchanged identity | Keep current session intent |
| Identity change | Do not reuse the previous session’s offset as preserve |
```

```
  open / switch session     ──►  latest (stick on)
  area navigate only        ──►  restore this session’s memory
  same identity + layout    ──►  preserve
  identity changed          ──►  no cross-session preserve
```

## Relationship to other chat behavior

Mid-turn streaming still follows stick-to-bottom while the user is following the live
answer; scrolling up to read history still pauses pure-content rebuild under the viewport.
Keyboard history and answer jumps remain separate. Scroll offsets are session-local for
the running app; they are not required to survive restart.
