# Review: oneshot hint chips (post-implementation)

Post-implementation review after chip UX, budget/model followup, and hygiene. Core design
holds; a few stale product strings lag the new surface.

## Scope

Proposal, design, four cap deltas (default-prompts, fast-response, warm-runtime, claude),
seven steps, followup `01`, and touched duckboard/duckchat/claude-acp code. `ds check` /
`ds audit oneshot-hint-chips` clean; duckboard 343 + duckchat 92 tests green.

## Summary

```
| # | sev | lens | title | → next |
| --- | --- | --- | --- | --- |
| 1 | major | fidelity | Settings still describe under-input Cmd-Enter hints | /ds-step |
| 2 | minor | quality | Stale “under-input” comments on session/config fields | /ds-step |
| 3 | minor | soundness | Oneshot chip clear-before-send if no agent handle | ignore |
```

## Findings

### 1. Settings still describe under-input Cmd-Enter hints - fidelity/major

**Where:** `crates/duckboard/src/area/settings.rs:161-175`

**Why:** Feature is fast-response chips when eligible, not under-input + Cmd-Enter. Wrong
copy misleads anyone enabling agent input hints and freezes a false mental model next to
correct deltas.

**Action:** Rewrite section/help to match settled product (chips, ⌘n, ghost exclusion,
default off).

### 2. Stale “under-input” comments on session/config fields - quality/minor

**Where:** e.g. `crates/duckboard/src/area/interaction.rs` oneshot list comment;
`crates/duckboard/src/config.rs` chat affordances blurb

**Why:** Low lasting cost but confuses the next reader; same drift class as settings.

**Action:** Align comments with “settled oneshot → chips” language in a tiny hygiene pass.

### 3. Oneshot chip clear-before-send if no agent handle - soundness/minor

**Where:** `crates/duckboard/src/area/interaction.rs` `activate_fast_response`
OneshotHints arm

**Why:** Shell/list cleared before `send_prompt_text`; without a handle, send no-ops and
chips vanish. Rare (chips only after a live turn) and not a new class of bug vs other send
paths.

**Action:** Accept or reorder clear-after-successful-send; not blocking archive.

## Open questions

- Followup issue 3 (live catalog → auto-cheapest oneshot) remains intentionally out of
  this change.

## Verdict

**Accept with a small fidelity fix.** Architecture (ghost > oneshot chips; question tool
wins; 30s budget; haiku preferred when advertised) matches proposal/design/specs. Archive
is reasonable after settings copy (and optional comment) cleanup; not after a redesign.
