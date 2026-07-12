# Question label prefix

Store and display host question chips as `Question: <text>` so settled history and live
chrome read clearly.

## Prerequisites

- [x] @step claude-permission-prompt

## Context

Followup #2: prepend `Question: ` when committing to the session and when rendering (live
can share the same formatter so live matches history).

## Tasks

- [x] 1. Add a small formatter (e.g. `format_user_choice_question_text`) that prefixes
         `Question: ` when missing and is idempotent if already present

- [x] 2. Use it in `settle_user_choice_transcript` when appending `UserChoiceQuestion`,
         and in live question chip display (`live_question_prompt` /
         `view_fast_response_question_chip`)

- [x] 3. Update settle and live-question unit tests so expected bodies include the
         `Question: ` prefix; keep existing `@spec` backlinks on those tests
