# Harness model picker

The model picker is where a chat's model is chosen. When more than one agent backend is
available, the picker keeps models legible by grouping them under their backend, and the
usage meter beside it reflects the chosen model's true context size.

## Grouped choices

Selectable models from every backend share one picker, but they are not merged into a flat
list. Each model is presented under its owning harness, so a reader can tell a grok model
from a Claude model at a glance and knows which backend a selection will run on.

## Usage meter

The usage meter shows how full the context is for the active chat. Fill is always measured
against the *selected model's* own context window — a 500k-window model and a 200k-window
model at the same token count read as different fill levels.

When the selected model's context window is unknown, the meter shows no fill rather than
guessing against a default window, so it never displays a misleading number.
