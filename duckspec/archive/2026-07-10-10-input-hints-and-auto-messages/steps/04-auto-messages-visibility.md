# Auto messages visibility

Gate obvious chrome visibility and when-visible key resolution on the auto messages flag.

## Prerequisites

- [x] @step chat-config-flags

## Tasks

- [x] 1. In `crates/duckboard/src/obvious_bubble.rs`, add `auto_messages: bool` to
         `chrome_visible` and the `resolve_cmd_*_when_visible` helpers; when false, chrome
         is hidden and keys resolve to no send

- [x] 2. Update existing unit tests to pass `auto_messages: true` where chrome should show

- [x] 3. @spec chat/obvious-bubble Chrome visibility: Idle empty composer with chrome shows chrome

- [x] 4. @spec chat/obvious-bubble Chrome visibility: Streaming hides chrome

- [x] 5. @spec chat/obvious-bubble Chrome visibility: Non-empty composer hides chrome

- [x] 6. @spec chat/obvious-bubble Chrome visibility: Empty chrome is hidden

- [x] 7. @spec chat/obvious-bubble Chrome visibility: Oneshot pending does not hide chrome when otherwise visible

- [x] 8. @spec chat/obvious-bubble Chrome visibility: Auto messages disabled hides chrome
