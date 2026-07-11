# Followup: composer key markers

User-led pass: agent-hint oneshot should use Cmd-Enter (binding + legible marker), not
Shift-Enter; multi-next tab marker should sit before the ghost for symmetry with the
oneshot row.

## Scope

Post–step-07 followup on `next-card-composer-hints`: oneshot under-input send binding and
marker; empty-composer next-action ghost / tab-available chrome. Checked code, design
table, default-prompts scenarios, and settings copy.

## Summary

```
| # | sev | lens | title | → next |
| --- | --- | --- | --- | --- |
| 1 | major | fidelity | Oneshot should be Cmd-Enter (bind + marker) | /ds-spec |
| 2 | minor | quality | Tab marker before ghost (mirror oneshot) | /ds-step |
```

## Issues

### 1. Oneshot should be Cmd-Enter (bind + marker) - fidelity/major

**Where:** `crates/duckboard/src/widget/text_edit/render.rs` (Enter: empty Shift-Enter →
`on_empty_shift_submit`; ⌘Enter uncaptured); `ONESHOT_SHIFT_ENTER_MARKER` (`⇧↩`);
`view_oneshot_suggestion`; settings strings; design empty-composer bindings;
`caps/chat/default-prompts` oneshot send/presentation scenarios

**Why:** Product wants agent input hints sent with empty **Cmd-Enter**. Current code,
design, specs, and settings all use **Shift-Enter**, and the `⇧↩` marker renders poorly in
content monospace (broken arrow glyphs). Marker, binding, and docs must agree on ⌘Enter;
glyph polish alone would lock in the wrong key.

**Action:** Spec empty Cmd-Enter for armed oneshot send (and no-op when unarmed /
pending); presentation marker as legible ⌘-enter affordance (UI font / reliable glyph,
e.g. `⌘↩` family with chrome). Implement TextEdit empty-⌘Enter path; stop using empty
⇧Enter for oneshot (non-empty ⇧Enter stays newline). Update settings copy. `/ds-spec` then
`/ds-step` / `/ds-apply`.

### 2. Tab marker before ghost (mirror oneshot) - quality/minor

**Where:** `crates/duckboard/src/widget/agent_chat.rs` (ghost placeholder + trailing `⇥`
tab slot); next-action multi chrome

**Why:** Oneshot uses key-before-text under the input. Multi next currently puts `⇥` on
the trailing edge of the input row, so the two empty-composer indicators are asymmetric
and the tab affordance is easy to miss relative to the ghost.

**Action:** When the tab-available marker is shown, place it before the ghost text (same
“key then affordance” pattern as oneshot). Keep visibility rules (`len > 1`, empty input,
idle) and avoid focus regressions from layout shape changes. `/ds-step` / `/ds-apply`
(after or with issue 1).

## Outcome

Agreed: switch oneshot to Cmd-Enter (specs + bind + marker), and put tab marker before
ghost for symmetry. Suggested next: `/ds-spec` (oneshot key scenarios), then `/ds-step`.
Not archive-ready until both land.
