# Align review template and enter-chip comments

Fix the stale `/ds-review` handoff line and Confirm/Commit-only comments so product
surfaces match the review-aware ladder and Create change affirm.

## Prerequisites

- [x] @step gate-steps-and-review-aware-ladder

## Context

Addresses findings in `reviews/01-post-implementation-chrome-ladder.md`:

- Review template claims reviews never change orientation next stage (now false).
- `theme.rs` / `agent_chat.rs` enter-chip comments omit Create change.

## Tasks

- [x] 1. Update Handoff in `crates/duckspec/content/templates/review.md`: replace the line
         that says a review never changes orientation next stage with wording that matches
         the review-aware ladder (a review may change chrome/orientation next stage; archive
         remains available when there are no open steps)

- [x] 2. Grep `crates/duckspec/content/templates/` for sibling “never changes” / “next stage”
         claims about reviews; fix any leftovers

- [x] 3. Update `crates/duckboard/src/theme.rs` `chat_obvious_chip_enter` doc comment to
         mention Create change alongside Confirm / Commit

- [x] 4. Update `crates/duckboard/src/widget/agent_chat.rs` Enter-tone / affirm chip comment
         to mention Create change alongside Confirm / Commit
