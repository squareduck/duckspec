# Chip UI and keybinds

Render key-first action chips (not faux user bubbles) and bind ⌘↩ / ⌘⌫ / ⌘1…9 to resolved
send text when chrome is visible.

## Prerequisites

- [x] @step session-field-and-soft-hint

## Context

Step 03 already stores `AgentSession.obvious_chrome`, refreshes it, and threads
`&obvious_chrome` into `agent_chat::view`. Temporary UI showed a **single** chip with the
⌘↩ target only (`resolve_cmd_enter`). This step replaces that with the full multi-chip
layout and ⌘⌫ / ⌘1…9 bindings.

## Tasks

- [x] 1. Replace single `view_obvious_bubble` with lifecycle chips plus gate row
         (Confirm/Reject pair or Commit alone); labels from pure helpers; click sends
         action string only

- [x] 2. Drop any remaining single-command view paths; confirm view takes `&ObviousChrome`
         end-to-end (largely done in step 03)

- [x] 3. Extend chat key path: ⌘↩ → `resolve_cmd_enter`, ⌘⌫ → `resolve_cmd_backspace`,
         ⌘1…9 → `resolve_cmd_digit`; only when `chrome_visible`; dispatch one send message
         into `send_prompt_text`

- [x] 4. Remove obsolete single-bubble activation leftovers; ensure oneshot pending does
         not hide chrome

- [x] 5. Smoke-check styling (muted chips, not user-bubble paper) and that empty Enter /
         Tab still use default-prompts only

- [x] 6. Run `cargo test -p duckboard` (or targeted modules) and fix regressions
