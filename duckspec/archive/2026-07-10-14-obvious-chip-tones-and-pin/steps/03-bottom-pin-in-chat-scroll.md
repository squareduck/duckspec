# Bottom pin in chat scroll

Wire viewport-driven chrome top pad through session scroll state and the chat view so
short history pins chips above the composer inside the scroll column.

## Prerequisites

- [x] @step numbered-tone-and-chrome-view

## Tasks

- [x] 1. Add ephemeral `chat_viewport_height`, `chat_content_height`, and `chrome_top_pad`
         on `AgentSession` (default pad `0.0`, heights `None`; not persisted)

- [x] 2. On `ChatScrolled`, store viewport/content heights and recompute `chrome_top_pad`
         via `chrome_bottom_pad`; zero the pad when chrome is not visible

- [x] 3. Thread `chrome_top_pad` into `agent_chat::view` and push a `Space` of that height
         above chrome when chrome is visible and pad > 0

- [x] 4. Update all `agent_chat::view` call sites to pass the pad

- [x] 5. Smoke-check: empty/short transcript pins chips near the input; long transcript
         keeps pad at 0 with chips after the last message
