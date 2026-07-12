# Other write-gate templates

Update every remaining write-gate template to the design token vocabulary and drop reason
text on decision tokens.

## Prerequisites

- [x] @step style-gate-token-rules

## Context

Token table from design: explore/backfill `create change <name>` / `reject change`;
explore `write project.md`; propose `confirm proposal`; design `confirm design` /
`reject design`; step `confirm steps` / `reject steps`; review `confirm review` /
`reject review`; followup `confirm followup` / `reject followup`; codex `confirm entry` /
`reject entry`; archive `confirm archive` / `reject archive`. Slash handoffs keep short
reasons. `apply.md` / `verify.md` only if they emit bare confirm chips.

## Tasks

- [x] 1. Update write-gate `next` cards and “wait for confirm” instructional prose in
         explore, backfill, propose, design, step, review, followup, codex, and archive
         under `crates/duckspec/content/templates/`

- [x] 2. Ensure decision-token lines have no trailing reason; leave slash-command handoff
         reasons intact

- [x] 3. Spot-check apply and verify templates for any bare `confirm` / `reject` chips and
         fix if present
