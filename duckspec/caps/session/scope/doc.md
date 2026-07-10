# Session scope orientation

The orientation duckspec hands a coding agent at the start of a session — identifying the
active scope, and for a change, the change it must act on by default, its progress, and
its suggested next stage — delivered reliably on the session's first turn.

Every chat session belongs to one scope. The orientation is a short blurb prepended to the
session's first turn so the agent knows where it is working without having to ask or
re-derive it from project state.

## Scope kinds

A session's scope is one of four kinds. The orientation it produces is tailored to the
kind.

```
kind          orientation content
────────────  ──────────────────────────────────────────────────────────────
change        the change name, project-root path under duckspec/changes/,
              its progress, its next stage, and a statement that
              change-acting commands target this change by default
exploration   an early-stage brainstorming chat with no formal artifacts yet
caps          the project's capability tree — points at duckspec/caps/ and
              duckspec/project.md
codex         the project's codex — points at duckspec/codex/ and
              duckspec/project.md
```

Only the change kind carries progress and a next-stage suggestion. The other kinds
describe their scope and nothing more — they never report change progress or a change
next-stage.

## Change orientation

For a change scope the orientation is authoritative: it names the change, states that
change artifacts live under the project-root path `duckspec/changes/{name}/`, and tells
the agent that change-acting commands — such as archive and apply — act on that change by
default. The agent disambiguates only when the user names a different change, so a project
with several active changes never forces the agent to ask which one to act on.

The orientation also reports where the change sits in its lifecycle: the suggested next
stage and step progress.

## Lifecycle and next stage

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

## Step progress

Progress is reported at the step level: how many steps are complete out of the total. For
the single step currently in progress, the orientation may also note that step's task
tally — how many of its tasks are done. A change whose steps are all complete is reported
as complete.

## Delivery

The orientation rides the body of the session's first turn, so it reaches the agent
reliably. It is present on that first turn whether or not the project has an `AGENTS.md`
file. Once the first turn has been sent, the orientation is not repeated — later turns in
the same session already carry it in their history.

## Current review

A change scope's orientation also reports the change's current review — the
highest-numbered review the change holds — as the project-root path
`duckspec/changes/{name}/reviews/{filename}`. When the change has no reviews, the
orientation says nothing about a current review.

The current review path is informational. Step progress counts are derived only from step
completion. The suggested next stage follows the shared review-aware lifecycle, so a
review may change next stage (for example all-complete with a review suggests step for
rework, with archive still available later as a chrome option).
