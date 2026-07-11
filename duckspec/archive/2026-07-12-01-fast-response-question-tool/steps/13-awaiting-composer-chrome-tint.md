# Awaiting composer chrome tint

While awaiting a user choice, apply a quiet accent tint (same treatment as numbered option
chips) to the whole composer section, including the model selector, so custom-answer mode
is visible and the model control does not stand out.

## Prerequisites

- [x] @step duckboard-freeform-as-custom-answer

## Tasks

- [x] 1. Theme/style helpers for awaiting composer tint in `crates/duckboard/src/theme.rs`
         (quiet accent, matching numbered fast-response chips)

- [x] 2. Apply tint to the composer section while `is_awaiting_user`; clear when not
         (`crates/duckboard/src/widget/agent_chat.rs` or equivalent)

- [x] 3. Model selector uses the same awaiting tint so it matches the composer section

- [x] 4. @spec chat/fast-response Awaiting composer chrome: Awaiting user applies quiet accent tint to the composer section

- [x] 5. @spec chat/fast-response Awaiting composer chrome: Not awaiting leaves the composer section untinted

- [x] 6. @spec chat/fast-response Awaiting composer chrome: Model selector matches the composer section tint while awaiting
