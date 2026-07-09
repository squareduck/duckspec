# Segment presentation helpers

Produce collapsed labels and expanded activity rows (status, summary, truncated output)
from transcript segments — pure helpers with no nested per-tool expand state.

## Prerequisites

- [ ] @step segment-builder-construction-and-pairing

## Tasks

- [x] 1. Add collapsed Thinking label helper (line count, no duration) and collapsed
         Activity summary (count + sample names)

- [x] 2. Shape expanded Activity presentation as one quiet row per tool with status,
         summary, and truncated output under the row

- [x] 3. @spec chat/transcript Segment presentation: Thinking collapsed label includes line count

- [x] 4. @spec chat/transcript Segment presentation: Activity collapsed label includes count and sample names

- [x] 5. @spec chat/transcript Segment presentation: Expanded activity exposes status, summary, and truncated output
