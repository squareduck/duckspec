# Equal chat / content split

By default, the interaction column and content column share free width equally; after the
user first resizes the panel, width stays absolute on resize. Chat already fills when
there is no content column to show.

## Motivation

The chat panel defaults to a fixed pixel width. On large windows the content column takes
almost everything; on smaller ones chat and content fight for leftover space. Resize never
rebalances an uncustomized panel, so “default” does not mean “balanced.”

Why now: the three-column layout and door handle are stable. Fixing the default split
before more chat chrome lands avoids teaching a permanent bias toward a fixed chat width.

## Intent

- Uncustomized panels split free width 50/50 between content and chat (free space after
  sidebar, list, and handle)

- Window resize and other default layout cases recompute that equal split while the panel
  is uncustomized

- The first middle-grip drag marks the panel customized; after that, width stays absolute
  pixels on resize

- When there is no content column (exploration with no tabs, or manual content collapse),
  chat fills the remaining space — same as today

- Uncustomized mode is not limited by the current fixed max chat width so true half-width
  is possible on large screens

- Customization is session memory only; no durable width preference yet

## Non-goals

- Persisting custom or default width across restarts

- Changing when the content column is shown or hidden beyond current exploration-no-tabs
  and manual collapse

- Ratio-preserving resize after the user has customized width

- Redesigning the door handle affordances (top toggle, middle drag, bottom collapse)

- Equal-width behavior for the list column or dashboard layout
