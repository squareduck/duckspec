# Cmd-Enter oneshot and tab-before-ghost

Wire empty Cmd-Enter for oneshot (drop empty Shift-Enter oneshot path); put tab marker
before ghost; settings copy.

## Prerequisites

- [x] @step oneshot-freeform-and-shift-enter
- [x] @step ghost-clear-and-focus-on-send

## Context

From followup `reviews/03-followup-composer-key-markers.md` and updated `default-prompts`
deltas: oneshot send is empty **Cmd-Enter** with legible `⌘↩` marker (marker string
already `ONESHOT_CMD_ENTER_MARKER`); multi-next tab marker must sit **before** the ghost.
Spec scenarios already linked on step 04.

## Tasks

- [x] 1. In `text_edit`, fire oneshot submit on empty Cmd-Enter (not empty Shift-Enter);
         keep non-empty Shift-Enter as newline; leave empty Shift-Enter as no-op for
         oneshot

- [x] 2. Wire chat input to the Cmd-Enter oneshot path (`SendOneshotSuggestion` /
         helpers); rename `on_empty_shift_submit` / comments as needed for clarity

- [x] 3. Place the tab-available marker before the next-action ghost (key then affordance,
         symmetric with oneshot); keep stable input-row focus behavior

- [x] 4. Update settings copy to say Cmd-Enter (not Shift-Enter) for agent input hints
