# Close review to archive handoff loop

Make the happy path apply-done → review → archive rank archive again after a review, and
keep the shipped archive commit handoff project-neutral.

## Context

Addresses findings in `reviews/01-post-implementation-orientation-paths-and-handoffs.md`:
archive never ranked after the user takes review; archive commit handoff hardcodes this
project's jj conventions.

## Tasks

- [x] 1. In `crates/duckspec/content/templates/review.md` Handoff: when findings need
         work, ① `/ds-spec` or `/ds-step` by finding type (no secondary); when the change
         is ready to accept (no open findings / verdict accepts finishing) and work is
         complete enough to archive, ① `/ds-archive`. Keep ≤2 total ranks; offer once.

- [x] 2. In `crates/duckspec/content/templates/archive.md` Handoff: propose a commit
         message from project conventions / `AGENTS.md` when present; wait for explicit
         confirmation; never auto-commit. Remove the "this repo uses jj" / hard-coded
         commit-grammar clause.

- [x] 3. Spot-check both Handoffs still obey the hard ≤2 ranked rule and do not
         reintroduce `/ds-verify`.

- [x] 4. Sanity-read the apply Handoff: ① review ② archive when all steps complete
         remains; no conflict with the new review→archive primary.
