# @ Session scope orientation

## ~ Lifecycle and next stage

```
change state                                  suggested next stage
────────────────────────────────────────────  ────────────────────
proposal only                                  design
design, no specs                               spec
specs, no steps (no reviews)                   step
open steps (no reviews)                        apply
open steps + review                            apply
all steps complete (no reviews)                archive
no open steps + review                         step
```

Suggested next stage is the first option of the same lifecycle list obvious chrome uses,
including when a review file is present.

## ~ Current review

A change scope's orientation also reports the change's current review — the
highest-numbered review the change holds — as the project-root path
`duckspec/changes/{name}/reviews/{filename}`. When the change has no reviews, the
orientation says nothing about a current review.

The current review path is informational. Step progress counts are derived only from step
completion. The suggested next stage follows the shared review-aware lifecycle, so a
review may change next stage (for example all-complete with a review suggests step for
rework, with archive still available later as a chrome option).
