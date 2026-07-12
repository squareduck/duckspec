# @ Content / chat column split

## ~ Modes

```
┌─ uncustomized ─────────────────────────────────────────┐
│ content and interaction each get half of free space    │
│ resize, door open, and programmatic force-show         │
│ recompute half from the current window                 │
│ new panel with a known window starts at that half      │
└───────────────────────────┬────────────────────────────┘
                            │ first grip width change
                            ▼
┌─ customized ───────────────────────────────────────────┐
│ interaction width is absolute pixels                   │
│ resize keeps that width; no equal rebalance            │
└────────────────────────────────────────────────────────┘

┌─ content hidden ───────────────────────────────────────┐
│ no open tabs (any three-column area), or content       │
│ collapsed                                              │
│ interaction fills remaining width (not the half value) │
│ stored width is only the restore target for next split │
│ opening a list item that creates a tab shows content   │
└────────────────────────────────────────────────────────┘
```

## ~ Grip vs other chrome

```
| Action | Effect on width mode |
| --- | --- |
| Middle-grip drag that changes width | Marks customized; sets absolute width |
| Top chevron open/close | Visibility only; does not customize; uncustomized open rebalances from current window |
| Programmatic force-show | Visibility only; does not customize; uncustomized open rebalances from current window |
| Bottom chevron collapse/restore content | Content visibility only; does not customize |
```

Drag width is clamped between the minimum panel width and free space so the row does not
invert. Uncustomized half-width is not limited by a fixed maximum below free space.
