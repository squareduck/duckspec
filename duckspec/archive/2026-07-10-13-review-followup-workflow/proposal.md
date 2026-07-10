# Review and followup workflow

Make change critique scannable and dual-mode: agent-led `/ds-review` and user-led
`/ds-followup`. Both append kind-prefixed history under `reviews/`, hand off rework to
later stages, and stay on lifecycle chrome mid-steps and after completion.

## Motivation

Reviews already capture judgment static tooling cannot, but two gaps hurt daily use.
First, the written artifact is a wall of prose while only the chat handoff shows a triage
table — reopening a review later is hard to scan. Second, the lifecycle treats “a review
exists” as end of critique: open steps drop `/ds-review`, and there is no first-class path
for user-led course correction that leaves a clear history record before archive. After
implementation (and during it), both agent scan and human-led followup need to find
issues, recommend next stages (`/ds-spec`, `/ds-step`, …), and preserve that judgment in
the change’s `reviews/` log.

## Scope

```
caps/
├── review/                 (modified — kind prefixes, dual critique modes)
├── chat/
│   └── obvious-bubble/     (modified — review + followup on lifecycle arms)
└── session/
    └── scope/              (modified — next-stage aligns with new ladder)
```

### New capabilities

- None. Followup records share the existing `reviews/` document recognition; divergence
  lives in schema, template, and filename kind — not a parallel artifact kind or
  directory.

### Modified capabilities

- `review` — Critique records under a change's `reviews/` stay document-schema,
  append-only, and sequentially numbered. New files use kind-prefixed slugs:
  `NN-review-<slug>.md` and `NN-followup-<slug>.md`. Create paths exist for both kinds.
  Both modes find issues (agent-led vs user-led) and record recommended next stages;
  applying plan or code changes is a later choice (`/ds-spec`, `/ds-step`, `/ds-apply`, or
  an explicit post-doc in-place fix). Legacy unprefixed filenames remain recognized. Body
  shape becomes scannable (summary table plus structured detail headings); hard validation
  of that body stays out of `ds check`.

- `chat/obvious-bubble` — Lifecycle chrome offers `/ds-review` and `/ds-followup` whenever
  steps are open and when steps are complete (or there are no open steps). Presence of
  existing reviews does not remove those chips (re-review and re-followup). Default order:
  open steps → apply, review, followup; all done → archive, review, followup; rework path
  keeps step/spec and archive while still listing review and followup.

- `session/scope` — Orientation suggested next stage stays the first option of the same
  lifecycle list as obvious chrome, including the dual-critique arms.

### Out of scope

- New `ArtifactKind` or a parallel `followups/` directory
- Hard schema enforcement of findings tables or structured fields (recommended shape only)
- Migrating historical/archived review filenames to add kind prefixes
- Implementing product code inside review or followup (still `/ds-apply`)
- Duckboard list UI kind badges beyond lifecycle chips and orientation

## Impact

```
  /ds-review (agent) ──┐
                       ├──→ reviews/NN-{review|followup}-<slug>.md
  /ds-followup (user) ─┘              │
                                      │ recommend next stage
                                      ▼
                         /ds-spec · /ds-step · /ds-apply · /ds-archive
                         (plan/code changes outside critique write gate)

  open steps:   apply, review, followup
  all done:     archive, review, followup
  rework path:  step, spec, … review, followup, archive
```

- **duckpond:** `create_review` / create-followup planning with kind-prefixed paths;
  `STAGES` gains `followup`; review cap scenarios for naming and create

- **duckspec content:** scannable `schemas/review.md`, peer `schemas/followup.md`;
  rewritten `templates/review.md`, new `templates/followup.md` (stock template section
  shape); command wrappers; `ds create followup` (or equivalent); reply-suggest lifecycle
  list includes followup

- **duckboard:** `change_scope_facts` lifecycle arms and tests; obvious-bubble and
  session/scope caps updated in lockstep

- No breaking API for consumers beyond chrome ladder and new create/template surfaces;
  existing `reviews/*.md` files keep validating as documents
