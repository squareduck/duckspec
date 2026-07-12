# Content / chat column split

Default equal free-space width for the content and interaction columns until the user
customizes via grip drag; absolute width after that; full remaining width for the
interaction column when content is hidden.

## Free space

In three-column areas, free horizontal space for the content ↔ interaction split is the
window width minus:

- sidebar and its vertical divider
- list column and its vertical divider
- interaction handle (when the handle is shown)

The list column stays fixed-width. Equal split applies only to content and interaction.

## Modes

```
┌─ uncustomized ─────────────────────────────────────────┐
│ content and interaction each get half of free space    │
│ resize / open recompute half from current window       │
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

Customization is session memory only. It is not written to config or project files.

## Grip vs other chrome

```
| Action | Effect on width mode |
| --- | --- |
| Middle-grip drag that changes width | Marks customized; sets absolute width |
| Top chevron open/close | Visibility only; does not customize |
| Bottom chevron collapse/restore content | Content visibility only; does not customize |
```

Drag width is clamped between the minimum panel width and free space so the row does not
invert. Uncustomized half-width is not limited by a fixed maximum below free space.
