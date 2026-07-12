# Session scroll policy in update wrapper

Add chat identity, snap-to-latest, open/switch classifier, and rewrite
`update_with_scroll_preservation` so open/switch snaps latest, area nav restores, and
layout preserve never crosses session identity; clean redundant restore on open/switch
paths.

## Tasks

- [x] 1. Add `ChatIdentity` and `active_chat_identity` in `crates/duckboard/src/main.rs`

- [x] 2. Add `snap_chat_to_latest` (set stick-to-bottom on the active session, issue
         `snap_to_end`) next to `restore_chat_scroll`

- [x] 3. Add `message_opens_or_switches_chat` closed match per design (session tab
         new/select/clear, idea/change list select, dashboard open change/exploration,
         open idea/change cross-links, add/start exploration)

- [x] 4. Rewrite `update_with_scroll_preservation`: capture identity before/after
         `update`; same identity → existing snapshot preserve; identity change +
         `AreaSelected` → restore only (no prior-session replay); identity change +
         open/switch classifier → `snap_chat_to_latest`; other identity changes → restore
         without replaying prior snapshot

- [x] 5. On open/switch paths that currently call `restore_chat_scroll`, remove the
         redundant restore when the wrapper will snap (keep restore on pure `AreaSelected`
         and incidental area entry)

- [x] 6. Keep find/landmark `chat_scroll_overridden` and pending chat autoscroll early
         exits unchanged
