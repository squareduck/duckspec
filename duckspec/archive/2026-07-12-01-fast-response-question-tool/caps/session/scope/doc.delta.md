# @ Session scope orientation

## ~ Lifecycle and next stage

```
change state                                  suggested next stage (first option)
────────────────────────────────────────────  ────────────────────
proposal only                                  design
design, no specs                               spec
specs, no steps (no reviews)                   step
open steps (with or without reviews)           apply
all steps complete (no reviews)                archive
no open steps + review                         step
```

Suggested next stage is the first option of the review-aware lifecycle ladder. That full
list also offers critique modes mid-steps and after completion:

```
open steps:
  apply, review, followup

all steps complete, no reviews:
  archive, review, followup

no open steps + review:
  step, spec, review, followup, archive
```

Presence of a review file does not remove `/ds-review` or `/ds-followup` from the ladder;
it selects the rework-aware arm when there are no open steps (first option `step`).

## @ Current review

A change scope's orientation also reports the change's current review — the
highest-numbered file under `reviews/` — as the project-root path
`duckspec/changes/{name}/reviews/{filename}`. That includes kind-prefixed files
(`NN-review-…`, `NN-followup-…`) and legacy unprefixed names. When the change has no
reviews, the orientation says nothing about a current review.

The current review path is informational. Step progress counts are derived only from step
completion. The suggested next stage follows the shared review-aware lifecycle, so a
review may change next stage (for example all-complete with a review suggests step for
rework, while archive, review, and followup remain available later on the ladder).
