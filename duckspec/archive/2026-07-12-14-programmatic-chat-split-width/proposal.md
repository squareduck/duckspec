# Programmatic chat split width

When chat is opened by app code rather than the door handle, content and chat should still
share free space equally until the user customizes the grip.

## Motivation

Equal free-space width for content and chat is already the product rule when the
interaction panel is uncustomized. Door open and window resize recompute half width from
the live window. Programmatic opens do not: idea Explore force-shows a newly created
panel, and other force-visible paths can leave chat pinned to a default-window half while
content takes the rest. On any window that is not that default size, the first Explore
(and similar opens) look wrong until a resize or door cycle.

Why now: Explore is the sharpest case of a broader “programmatic open” gap; fixing every
force-visible path and how new interaction state is sized keeps the equal-split rule
honest without a second layout story.

## Intent

- Every programmatic open that sets the interaction panel visible applies the same
  uncustomized equal-width rule as door open, using the current window width

- A newly created interaction panel is sized from the live window width, not only from a
  fixed default window size

- With content shown and the panel uncustomized, content and chat remain equal free-space
  halves after those opens

- Grip customization, content-hidden fill, and resize behavior stay as they are today

## Non-goals

- Changing free-space geometry, grip customization, or content-hidden fill rules

- Redesigning idea Explore, change selection, or exploration creation beyond correct width
  on open

- Persisting split width across restarts

- New user-facing controls for the split
