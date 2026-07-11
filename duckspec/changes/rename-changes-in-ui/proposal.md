# Rename changes in UI

CHANGE sidebar rows can be renamed by hand, and a refresh control can regenerate a title
from the active session’s chat so labels are not stuck on “Exploration N” or a stale
first-shot summary.

## Motivation

Explorations and change rows in the CHANGE list are hard to tell apart once several exist.
Default names (`Exploration 6`) only improve if the first-turn title oneshot runs — and
that path never retitles once a title is set. Users need an explicit way to set the label
and to ask for a fresh AI title after the conversation has moved on.

## Intent

- From the CHANGE list, the user can rename the selected exploration’s display name
  without leaving the list

- A refresh control regenerates a title from the **active session’s current chat** (not
  only the first user message, and not blocked by an existing title) and applies it to
  that session’s title

- For explorations, a successful rename or refresh updates the row label (`display_name`)
  so the list matches the session

- Refresh is unavailable or a no-op when there is nothing to summarize (empty /
  bare-slash-only chat, missing session, or in-flight stream), without wiping the existing
  label on failure

- Manual rename and refresh write the same label field; whichever runs last wins

## Non-goals

- Renaming real change folders (`duckspec/changes/<slug>/`) or archived entries
- Auto re-title on every turn or background retitle without a user click
- Renaming session dropdown entries for caps/codex scopes
- New CLI (`ds rename`) or duckpond plan types
- Composer-token or keybind redesign beyond the CHANGE-list affordances
