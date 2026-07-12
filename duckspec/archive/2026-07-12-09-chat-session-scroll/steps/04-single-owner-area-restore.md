# Single-owner area restore

Make area-nav restore owned in one place: wrapper issues `restore_chat_scroll` on
area-only identity change; remove restore from the `AreaSelected` update arm (or document
a hard invariant if keeping dual ownership).

## Prerequisites

- [x] @step strengthen-session-scroll-scenario-tests

## Context

Review finding 2 (minor): `AreaRestoreIssued` assumes `Message::AreaSelected` already
returned `restore_chat_scroll`. Prefer single owner per design.

## Tasks

- [x] 1. Change `chat_scroll_policy` / wrapper so area-only identity change issues
         `restore_chat_scroll` in the wrapper (replace `AreaRestoreIssued` no-op with
         `Restore`, or an explicit `AreaRestore` that still calls restore)

- [x] 2. Remove `restore_chat_scroll` from the `Message::AreaSelected` arm in `update` so
         restore is not dual-owned

- [x] 3. Confirm area round-trip tests from step 03 still pass; adjust if restore timing
         depended on the update arm
