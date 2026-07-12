# Suppress AskUserQuestion activity

Do not show AskUserQuestion (or equivalent) as host Activity tool rows; chips own the
interaction surface.

## Prerequisites

- [x] @step question-label-prefix

## Context

Followup #3: Claude emits `tool_call` updates titled AskUserQuestion; the transcript
Activity card is noise next to option/question chips. Wire (permission / choice) must keep
working.

## Tasks

- [x] 1. Choose and implement the lightest filter: skip emitting AskUserQuestion tool_call
         host events and/or omit those tool names from `build_transcript_segments`
         Activity rows (document which layer in a one-line comment)

- [x] 2. Cover both Claude title shapes if needed (`AskUserQuestion` / humanized "Ask user
         question")

- [x] 3. Unit test: a session message with AskUserQuestion ToolUse does not produce an
         Activity tool row (or equivalent assert for the chosen filter layer)

- [x] 4. Confirm structured choice path still parks/answers (existing mid-prompt /
         freeform settle tests still pass)

- [x] 5. @spec chat/transcript Host-choice tools omitted from Activity: AskUserQuestion tool content is omitted from Activity
