# Followup: chips stay while typing custom answer; clear clippy

User-led pass after custom-answer + tint: live freeform works on both harnesses.
Remaining: keep option chips visible while typing a custom answer, and clean clippy
warnings.

## Scope

Live verify post-steps 11–13. Visibility in `crates/duckboard/src/fast_response.rs`
(`visible`) and `caps/chat/fast-response` Visibility. Clippy on `duckchat`,
`duckchat-claude-acp`, and `duckboard`.

## Summary

```
| # | sev | lens | title | → next |
| --- | --- | --- | --- | --- |
| 1 | major | fidelity | Chips hide when typing custom answer while awaiting | /ds-spec |
| 2 | minor | quality | Clippy warnings (duckchat / claude-acp / duckboard) | /ds-step |
```

## Issues

### 1. Chips hide when typing custom answer while awaiting - fidelity/major

**Where:** `crates/duckboard/src/fast_response.rs` `visible` (requires `input_empty`);
`caps/chat/fast-response` Visibility scenario “Non-empty composer hides chips”

**Why:** Composer is the custom-answer surface while awaiting; hiding chips on the first
keystroke removes ⌘n options mid-type. When not awaiting, non-empty input should still
hide chips so a later oneshot-hint fill does not compete with typed text.

**Action:** Spec: non-empty composer hides chips only when the session is *not* awaiting a
user choice; while awaiting, chips remain visible with a non-empty composer. Implement
`visible` + tests via `/ds-spec` → `/ds-step`.

### 2. Clippy warnings (duckchat / claude-acp / duckboard) - quality/minor

**Where:** `duckchat` (collapsed ifs, MutexGuard held across await); `duckchat-claude-acp`
(collapsed ifs, redundant closure, complex type); `duckboard` (collapsed if)

**Why:** Noise on clean builds; not user-facing product behavior.

**Action:** Chore step: clear clippy for those packages (`cargo clippy` / `--fix` where
safe). No new capability requirements.

## Outcome

Custom answer + tint accepted live. Open: awaiting chips-while-typing (needs spec) and
clippy cleanup. Plan and code unchanged in this write. Not archive-ready until issue 1
lands (issue 2 can ship in the same implementation pass). Suggested next: `/ds-spec` then
`/ds-step`.
