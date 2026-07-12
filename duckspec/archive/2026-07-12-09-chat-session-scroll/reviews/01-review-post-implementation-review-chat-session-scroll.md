# Post-implementation review: Chat session scroll

Reviewed `chat-session-scroll` through proposal → design → `chat/session-scroll` caps →
steps → `main.rs` implementation and `@spec` tests. Policy matches intent; freeze is held
back by hollow scenario coverage.

## Scope

Change artifacts (`proposal.md`, `design.md`, `caps/chat/session-scroll`, steps 01–02),
`crates/duckboard/src/main.rs` scroll helpers and `update_with_scroll_preservation`, and
the six `@spec` tests. Post-implementation; audit 6/6 linked.

## Summary

```
| # | sev | lens | title | → next |
| --- | --- | --- | --- | --- |
| 1 | major | quality | `@spec` tests only exercise the pure decision table | /ds-step |
| 2 | minor | quality | Area restore dual-owned by `update` and wrapper | /ds-step |
| 3 | minor | quality | Incidental area switch may double-`restore_chat_scroll` | ignore |
```

## Findings

### 1. `@spec` tests only exercise the pure decision table - quality/major

**Where:** `crates/duckboard/src/main.rs` tests around
`intentional_session_open_or_switch_lands_at_latest` /
`area_change_restores_remembered_mid_history` (~6286–6422)

**Why:** Scenarios assert `chat_scroll_policy(…)` and `restored_viewport_intent` with
hand-set booleans. They never run `SelectIdea` / `SelectSession` / `AreaSelected` against
`State`, never assert stick/`last_chat_offset_y` after a real open, and cannot fail if the
wrapper stops calling `snap_chat_to_latest`. Audit stays green while the contract is
unfalsified.

**Action:** In a follow-up step, drive minimal `State` transitions (scope pick, session
tab, area round-trip) and assert stick flags / restored intent on the active
`AgentSession` after the policy path that production uses.

### 2. Area restore dual-owned by `update` and wrapper - quality/minor

**Where:** `Message::AreaSelected` → `restore_chat_scroll` (`main.rs` ~464) vs
`ChatScrollPolicy::AreaRestoreIssued` (~2970) which assumes that restore already ran

**Why:** If the `AreaSelected` arm drops restore later, mid-history area return silently
breaks while open/switch still looks fine. Design already preferred a single owner.

**Action:** Prefer one site: wrapper always `restore_chat_scroll` on area-only identity
change, or keep restore only in `update` and document the invariant next to
`AreaRestoreIssued`.

### 3. Incidental area switch may double-`restore_chat_scroll` - quality/minor

**Where:** open-file paths that `switch_area` + `restore_chat_scroll` (~3304, ~5034) and
wrapper `ChatScrollPolicy::Restore` on non-classified identity change

**Why:** Same intent issued twice per tick; low harm today, noisy if restore gains side
effects.

**Action:** Optional cleanup—either rely on the wrapper or skip restore in open-file when
identity will change.

## Verdict

The approach is sound and faithful: identity-gated preserve, snap on open/switch, restore
on pure area nav, classifier closed as designed. Not ready to freeze solely because
scenario tests do not falsify the integration path that users hit; fix finding 1 (and
optionally 2) before archive.
