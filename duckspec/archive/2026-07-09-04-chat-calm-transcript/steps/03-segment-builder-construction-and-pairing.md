# Segment builder construction and pairing

Implement pure `build_transcript_segments` that coalesces contiguous content into Thinking
/ Activity / Answer segments and pairs tools by call id.

## Prerequisites

- [ ] @step reasoning-content-model-and-persistence

## Tasks

- [x] 1. Introduce `TranscriptSeg`, `ToolRow`, and `build_transcript_segments` with
         contiguity rules, live pending buffers, and id-based tool pairing (orphan results
         as named done rows, never bare "done" alone) in
         `crates/duckboard/src/widget/agent_chat.rs` (or a sibling module)

- [x] 2. @spec chat/transcript Segment construction: Reasoning then answer yields Thinking then Answer

- [x] 3. @spec chat/transcript Segment construction: Contiguous tools yield one Activity with multiple rows

- [x] 4. @spec chat/transcript Segment construction: Thought, tools, thought, answer yields four segments in order

- [x] 5. @spec chat/transcript Segment construction: Live pending reasoning appears on an open Thinking segment

- [x] 6. @spec chat/transcript Activity pairing: Matching use and result become one done row

- [x] 7. @spec chat/transcript Activity pairing: Non-adjacent use and result still pair by id

- [x] 8. @spec chat/transcript Activity pairing: Orphan result is a named done row
