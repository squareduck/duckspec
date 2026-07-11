# Config and obvious chrome shell

Remove `auto_messages` from config and settings; collapse obvious chrome to `options` +
`cancel`, leave population empty, keep empty-send formatting and key/chip/pad helpers.

## Prerequisites

- [x] @step meta-card-parser

## Tasks

- [x] 1. Remove `ChatConfig.auto_messages` and the settings toggle; ensure old config keys
         load safely; drop all `auto_messages` parameters from call sites

- [x] 2. Refactor `ObviousChrome` to `options: Vec<String>` and `cancel: Option<String>`;
         update visibility, digit/cancel resolvers, chip labels, and `agent_chat` chrome
         view

- [x] 3. Make `refresh_obvious_chrome` / `build_obvious_chrome` leave options empty (still
         refresh `scope_facts` for orientation and bootstrap)

- [x] 4. @spec chat/obvious-bubble Empty-send option formatting: Bare skill name formats with leading slash

- [x] 5. @spec chat/obvious-bubble Empty-send option formatting: Already-slashed command is preserved

- [x] 6. @spec chat/obvious-bubble Ephemeral chrome: Visible chrome is not a stored user message

- [x] 7. @spec chat/obvious-bubble Chrome visibility: Idle empty composer with non-empty options shows chrome

- [x] 8. @spec chat/obvious-bubble Chrome visibility: Streaming hides chrome

- [x] 9. @spec chat/obvious-bubble Chrome visibility: Non-empty composer hides chrome

- [x] 10. @spec chat/obvious-bubble Chrome visibility: Empty options hide chrome

- [x] 11. @spec chat/obvious-bubble Key resolution: Cmd-digit sends matching option

- [x] 12. @spec chat/obvious-bubble Key resolution: Cmd-Backspace sends cancel when set

- [x] 13. @spec chat/obvious-bubble Key resolution: Resolution is a no-op when chrome not visible

- [x] 14. @spec chat/obvious-bubble Chip display: Option chip label is hotkey then action

- [x] 15. @spec chat/obvious-bubble Chip display: Cancel chip label is hotkey then cancel text

- [x] 16. @spec chat/obvious-bubble Chrome bottom pad: Short content yields positive pad

- [x] 17. @spec chat/obvious-bubble Chrome bottom pad: Content at or above viewport yields zero pad

- [x] 18. @spec chat/obvious-bubble Chrome population: Session chrome options are empty after refresh

- [x] 19. @spec chat/default-prompts Agent input hints gate: Default agent input hints setting is disabled

- [x] 20. @spec chat/default-prompts Agent input hints gate: Oneshot launch requires agent input hints enabled
