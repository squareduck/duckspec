# Orientation paths and workflow handoffs — Design

Fix project-root paths and the all-steps-complete next stage in the existing orientation
pipeline, and rewrite template Handoff sections to a fixed ranked ≤2 matrix. No new
modules or APIs.

## Approach

```
                    ┌─────────────────────┐
  ProjectData  ───→ │ change_scope_facts  │  next_command:
                    │  (area/change.rs)   │    all steps done → ds-review
                    └──────────┬──────────┘
                               │ ChangeScopeFacts
                               ▼
                    ┌─────────────────────┐
  SessionScope ───→ │ CurrentScopeHook    │  path strings:
                    │  render_*           │    duckspec/changes/{name}/
                    │  (scope.rs)         │    duckspec/caps|codex|project.md
                    └──────────┬──────────┘    duckspec/changes/{name}/reviews/{r}
                               │ first-turn body
                               ▼
                            agent

  templates/*.md Handoff  ──→  Primary / Secondary (optional)
       independent of orientation; same lifecycle story
```

Two thin touch surfaces share one lifecycle story:

- **`change_scope_facts`** picks the single primary stage for orientation and the composer
  placeholder (`obvious_command`).

- **Templates** state ranked ≤2 next actions, including secondaries orientation never
  carries (e.g. archive after review, propose after create change).

Orientation stays a short blurb: name, path, progress, one next stage, optional current
review, authority. Discovery (`ds status` / `ds index`) remains a template Context duty,
not part of the hook.

## change_scope_facts next-stage ladder

`change_scope_facts` in `crates/duckboard/src/area/change.rs` already derives
`ChangeScopeFacts { phase, steps_done, step_count, active_step_tasks,
next_command, current_review }`.
Only the all-steps-complete branch changes.

```rust
// crates/duckboard/src/area/change.rs — sketch
pub fn change_scope_facts(name: &str, project: &ProjectData) -> Option<ChangeScopeFacts> {
    // …
    if !change.steps.is_empty() {
        let all_done = steps_done == change.steps.len();
        // …
        next_command: Some(
            if all_done { "ds-review" } else { "ds-apply" }.into()
        ),
        // was: if all_done { "ds-archive" } else { "ds-apply" }
    }
    // remaining ladder unchanged:
    //   caps, no steps → ds-step
    //   design, no caps → ds-spec
    //   proposal only → ds-design
    //   empty → ds-propose
}
```

`refresh_obvious_command` / the chat placeholder already read `next_command` from these
facts — no separate wiring. Unit tests and the `session/scope` scenario that assert
archive-at-complete flip to review.

Orientation still exposes **one** next stage (the primary). Templates may also offer
archive as secondary when apply is complete; that secondary is not reflected in
`next_command`.

## CurrentScopeHook path copy

All path strings in `crates/duckboard/src/scope.rs` become project-root-relative under the
`duckspec/` directory (always named `duckspec/` for this tool).

```rust
// crates/duckboard/src/scope.rs — sketch
fn render_change_orientation(scope: &SessionScope) -> String {
    let name = &scope.scope_key;
    // authority line unchanged (still mentions ds status only for disambiguation)
    // …
    let review = match &facts.current_review {
        Some(r) => format!(
            " Current review: `duckspec/changes/{name}/reviews/{r}` (latest)."
        ),
        None => String::new(),
    };
    format!(
        "Current duckspec scope: change `{name}`. Change artifacts live under \
         `duckspec/changes/{name}/`. {progress}{next}{review} {authority}"
    )
}

// Non-change arms of CurrentScopeHook::compute:
// Caps:   … See `duckspec/caps/` and `duckspec/project.md`.
// Codex:  … See `duckspec/codex/` and `duckspec/project.md`.
// Exploration: unchanged in spirit (no change directory expected).
```

Facts-less change fallback uses the same `duckspec/changes/{name}/` path.

Unit tests in `scope.rs` and any string assertions elsewhere that expect `changes/{name}/`
or root `caps.md` / `codex.md` update accordingly. Spec/doc copy under `session/scope`
that documents paths or the lifecycle table (`all steps complete → archive`) updates to
match.

## session/scope delta

Capability contract changes are a delta only:

```
| Topic | Today | After |
|-------|--------|--------|
| Change artifact path | implied / bare `changes/{name}/` in code | SHALL use project-root path `duckspec/changes/{name}/` |
| Caps / codex pointers | root `caps.md` / `codex.md` / `project.md` | `duckspec/caps/`, `duckspec/codex/`, `duckspec/project.md` |
| Current review path | relative `reviews/{r}` | `duckspec/changes/{name}/reviews/{r}` |
| All steps complete next stage | archive | review |
```

Scenarios to retarget (names illustrative):

- All steps complete → suggests **review** (was archive).

- Add path scenarios (or extend identification scenarios) so orientation names
  `duckspec/changes/{name}/` and non-change scopes name the corrected paths.

Doc lifecycle table:

```
… incomplete steps → apply
all steps complete → review   // was: archive
```

## Template Handoff rewrites

Content-only under `crates/duckspec/content/templates/`. Not a capability. Shared rule in
every Handoff section:

- Offer **at most two** ranked next actions: **Primary**, then **Secondary**
  (omit secondary if none). Label them in plain text — not circled numerals.

- Offer once; if the user declines, drop it.

- Operational notes (Context propagation, Outcomes, audit interpretation) stay outside the
  ranked list — they are work rules, not next-stage suggestions.

### Ranked matrix

```
| Stage | State | Primary | Secondary |
|-------|--------|-----------|-------------|
| explore | no change for this work yet | create change | `/ds-propose` |
| explore | change already exists | `/ds-propose` | — |
| propose | written + validated | `/ds-design` | `/ds-spec` |
| design | open questions remain | resolve open questions | `/ds-spec` |
| design | no open questions | `/ds-spec` | `/ds-step` |
| spec | all scoped caps done | `/ds-step` | `/ds-archive` |
| step | steps written | `/ds-apply` | — |
| apply | unfinished steps remain | `/ds-apply` | — |
| apply | all steps complete, audit clean | `/ds-review` | `/ds-archive` |
| archive | success | commit (proposed message; wait for confirm) | — |
| review | findings need work | `/ds-spec` *or* `/ds-step` by finding type (one) | — |
| codex | mid-change | resume `/ds-<stage>` | — |
| codex | standalone | nothing | — |
| backfill | slice ready | `/ds-propose` | — |
```

### Archive commit suggestion

After successful archive + sync + audit report, the agent proposes a commit message in the
project's usual form (this repo: `type(optional-scope): …` via jj) and **waits for
explicit confirmation** — never auto-commits. Align wording with AGENTS.md commit rules
where present.

### Verify

`/ds-verify` remains a side skill/template. No main-flow Handoff points at it. No need to
delete the skill in this change.

## Decisions

- **Hardcode `duckspec/` prefix in orientation strings** — the tool's on-disk root is
  always the `duckspec/` directory at the project root. Alternatives: inject a
  configurable root path from project config (rejected for this change: no config surface
  today, and every template already hardcodes `duckspec/`).

- **Orientation carries only the primary next stage** — `next_command` stays a single
  `Option<String>`. Secondaries live only in templates. Alternatives: multi-value next
  stages in facts (rejected: bloats the first-turn blurb and the placeholder UI for little
  gain).

- **All-steps-complete → `ds-review`** — matches apply's primary after a clean finish;
  archive remains a template secondary and the post-review step. Alternatives: keep
  archive as orientation primary (rejected: fights the review-before-archive workflow).

- **Fully qualify current-review path** — same disambiguation fix as the change directory.
  Alternatives: leave `reviews/{r}` relative (rejected: agents still guess the parent).

- **Design open questions block progression** — template primary is resolve open questions
  when any remain; orientation still says `ds-spec` after design exists (facts cannot see
  open-question text without parsing design.md). Alternatives: parse design for open
  questions in `change_scope_facts` (rejected: fragile, out of scope for a string/facts
  tweak).

## Risks

- **Orientation says review while user wants to skip to archive** → templates still offer
  archive as **Secondary** after apply-complete; user can run `/ds-archive`
  directly. Orientation is a
  suggestion, not a gate.

- **Template-only handoff matrix drifts from orientation again** → keep the matrix table
  in this design and the proposal Impact section as the checklist when editing Handoffs;
  the all-done primary is the one place both must agree (`ds-review`).

## Open questions

None.
