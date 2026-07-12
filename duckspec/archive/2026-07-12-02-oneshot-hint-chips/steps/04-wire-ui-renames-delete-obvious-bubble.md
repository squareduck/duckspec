# Wire UI, renames, delete obvious-bubble

Hook oneshot settle and turn boundaries into chip sync, remove under-input oneshot UI,
rename leftover obvious/bubble identifiers, and delete the dead capability folder.

## Prerequisites

- [x] @step oneshot-fast-response-shell-and-activation

## Tasks

- [x] 1. In `widget/agent_chat.rs` (and related), remove under-input oneshot list/loading
         chrome and empty Cmd-Enter oneshot submit (`SendOneshotSuggestion` and handlers)

- [x] 2. On `TurnComplete`: after `refresh_next_actions`, only begin reply oneshot when
         launch gates pass (including empty next-action list); call `sync_oneshot_chips`
         as needed

- [x] 3. On `DefaultPromptsReady`: store settled list, clear pending,
         `sync_oneshot_chips`; on turn start / clear oneshot, re-sync empty

- [x] 4. Rename test-only `compute_obvious_command` / `obvious_command_from_artifacts` and
         `obvious_*` tests to lifecycle names; fix stale “obvious-command” comments in
         `main.rs` / `change.rs`

- [x] 5. Delete `duckspec/caps/chat/obvious-bubble/` (spec + doc); confirm no live
         `@spec chat/obvious-bubble` backlinks remain outside archive

- [x] 6. Run `cargo test -p duckboard -p duckchat` and
         `ds check duckspec/changes/oneshot-hint-chips` (or project check) for a clean
         step exit
