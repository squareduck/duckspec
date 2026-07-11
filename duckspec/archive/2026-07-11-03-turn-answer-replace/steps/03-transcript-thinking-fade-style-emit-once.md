# Transcript, Thinking fade, style emit-once

One live Answer with open Thinking for an uncommitted draft; Thinking body uses secondary
ink; shared style write-gate emit-once rule.

## Prerequisites

- [x] @step answer-draft-across-thought

## Tasks

- [x] 1. Ensure transcript segment build shows one live Thinking and one live Answer when
         both pending buffers are open (adjust only if replace/draft changes broke this)

- [x] 2. @spec chat/transcript Segment construction: Live reasoning with an open answer draft yields Thinking then one Answer

- [x] 3. Add optional `TextEdit` base color (default primary); set Thinking body to
         `theme::text_secondary()` in `view_thinking_block`

- [x] 4. Manually verify Thinking body is more faded than Answer body in light and dark
         (`chat/transcript` Thinking body fade)

- [x] 5. Add the surgical emit-once rule under Write gate in
         `crates/duckspec/content/schemas/style.md` (provider-neutral; no stage list)
