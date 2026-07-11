# @ Session scope orientation

## @ Requirement: Lifecycle reflection

For a change scope, the orientation SHALL report the change's step progress and a
suggested next stage that matches the change's artifact state, step completion, and
whether the change has any reviews — the first option of the review-aware lifecycle ladder
(including arms that also list `/ds-review` and `/ds-followup`). When steps remain
unfinished it SHALL report the incomplete progress; when every step is complete it SHALL
report completion.

## @ Requirement: Current review in orientation

For a change scope, the orientation SHALL report the change's current review — the
highest-numbered file under the change's `reviews/` directory (whether kind-prefixed as
review or followup, or a legacy unprefixed name) — as the project-root path
`duckspec/changes/{name}/reviews/{filename}` when the change has at least one review, and
SHALL omit any current-review report when the change has none. The presence of reviews
SHALL NOT change reported step progress (done and total counts). The suggested next stage
SHALL follow the review-aware lifecycle (same first option of that ladder), so a review
may change the suggested next stage relative to an otherwise identical change without
reviews.
