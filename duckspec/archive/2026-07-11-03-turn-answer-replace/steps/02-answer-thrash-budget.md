# Answer thrash budget

Count answer-after-thought replacements, cancel after the third, keep the last draft with
a short stop notice, ignore further content/reasoning until the turn ends; reset the
counter on tool use.

## Prerequisites

- [x] @step answer-draft-across-thought

## Tasks

- [x] 1. Add in-memory `answer_replace_count` on the session (not persisted); increment on
         each answer-after-thought replace; reset on tool use, turn complete, cancel
         settle, and new send

- [x] 2. When count exceeds 2, cancel the main agent path (same as user cancel), keep the
         last draft, append a short non-answer stop notice, and do not auto-start another
         turn

- [x] 3. Drop further answer/reasoning deltas after a thrash trip until streaming ends

- [x] 4. @spec chat/stream-ui Answer thrash budget: Third answer-after-thought cancels and keeps the last draft

- [x] 5. @spec chat/stream-ui Answer thrash budget: Tool use resets the thrash budget
