# Advisory review stage — Design

Add `review` as a `doc`-schema artifact under `changes/<name>/reviews/NN-<slug>.md`, a
`ds create review` command that appends the next number, a duckboard orientation field for
the current (highest-`NN`) review, and the two-movement `/ds-review` template and command
files.

## Approach

Reviews reuse every existing doc mechanism — parsing, validation, formatting, archival.
The genuinely new logic is small and isolated: one `ArtifactKind` plus its path
classification, an append-only `create_review` planner, and a `current_review` field
threaded through duckboard's session orientation. Phase and next-stage derivation are
deliberately left untouched — a review never gates a change.

```text
duckpond (lib)              duckspec (CLI)                 duckboard (orientation)
──────────────              ──────────────                 ───────────────────────
layout::ArtifactKind        cmd/create.rs                  data.rs
  ::Review ───classify──┐     CreateCommand::Review          ChangeData.reviews ──┐
plan::create_review     │   cmd/status.rs                   area/change.rs        │
  (append NN-slug)      │     review listing                  ChangeScopeFacts     │
check.rs                │   content/templates/review.md       .current_review ◀────┘
  Review → parse_doc ◀──┘   content/commands/*/ds-review.md   scope.rs
                            (ds init globs these in)            render_change_orientation
```

The data flow for "current review": duckboard's `data.rs` loads the `reviews/` directory
into `ChangeData.reviews`; `change_scope_facts` picks the highest-`NN` entry into
`ChangeScopeFacts.current_review`; `scope.rs` renders it into the first-turn orientation.
None of this touches `next_command`.

## Review artifact classification

A review is recognized purely by its path. `ArtifactKind` gains a `Review` variant and
`classify_change` routes the `reviews/` subdirectory the same way it routes `steps/`.

```rust
// duckpond/src/layout.rs
pub enum ArtifactKind {
    // … existing variants …
    /// `changes/<name>/reviews/NN-<slug>.md`
    Review,
}

fn classify_change(rest: &[&str]) -> Option<ArtifactKind> {
    // … proposal.md / design.md direct children …
    match within_change[0] {
        "caps" => classify_change_caps(&within_change[1..]),
        "steps" => classify_step(&within_change[1..]),
        "reviews" => classify_review(&within_change[1..]),
        _ => None,
    }
}

fn classify_review(rest: &[&str]) -> Option<ArtifactKind> {
    // Mirrors classify_step: single `.md` child, no nesting.
    (rest.len() == 1 && rest[0].ends_with(".md")).then_some(ArtifactKind::Review)
}
```

Steps and reviews share the `NN-<slug>.md` shape, so `extract_step_slug` is generalized to
a neutral helper both call:

```rust
// duckpond/src/layout.rs
pub fn extract_nn_slug(filename: &str) -> Option<String> { /* was extract_step_slug */ }
```

Archive support is free: `classify` already routes `archive/<dated>/…` through
`classify_change`, so an archived review classifies as `Review` with no extra code.

## Doc-schema validation

Reviews validate against the document schema, exactly like proposals, designs, codex
entries, and `project.md`. This is a one-line addition to the validator's doc arm.

```rust
// duckpond/src/check.rs — existing match
ArtifactKind::CapDoc
| ArtifactKind::ChangeCapDoc
| ArtifactKind::Proposal
| ArtifactKind::Design
| ArtifactKind::Codex
| ArtifactKind::Review        // ← new
| ArtifactKind::Project => parse::doc::parse_document(&elements) /* … */,
```

`build_context` in the CLI's `check.rs` needs no change: only `Step` carries a
`filename_slug`, and reviews — on the doc schema — don't enforce a slug/H1 match.

## create_review planner

Mirrors the append branch of `create_step`, minus `--after`: reviews are an immutable
chronological log, never reordered or inserted between.

```rust
// duckpond/src/plan.rs
pub fn create_review(
    name: &str,
    change: &str,
    active_changes: &[String],
    existing_reviews: &[String],   // filenames in changes/<change>/reviews/
) -> Result<Plan, PlanError> {
    check_change_exists(change, active_changes)?;
    let slug = slugify(name);
    let parsed = parse_nn_slug(existing_reviews);          // shared with steps
    if parsed.iter().any(|r| r.slug == slug) {
        return Err(PlanError::ReviewSlugExists { slug });
    }
    let next_nn = parsed.last().map_or(1, |r| r.nn + 1);
    Ok(Plan {
        creates: vec![review_path(change, next_nn, &slug)],
        renames: vec![],
    })
}
```

`PlanError` gains `ReviewSlugExists { slug }`. The existing `parse_steps` helper is
renamed `parse_nn_slug` (neutral) and shared; `review_path` mirrors `step_path`.

## ds create review command

```rust
// duckspec/src/cmd/create.rs
pub enum CreateCommand {
    // … existing …
    /// Create a review file in a change.
    Review {
        name: String,
        #[arg(long = "in")]
        change: String,
    },
}
```

The dispatch lists `changes/<change>/reviews/` and calls `duckpond::plan::create_review`.
One wrinkle: steps and reviews share the `NN-<slug>.md` filename shape, so
`placeholder_for(filename)` cannot tell them apart to emit `# Review` vs `# Step`. The fix
is to thread the placeholder title from the command (which knows it is creating a review)
rather than sniff the filename — e.g. an optional title override carried alongside the
`Plan`, applied in the create loop the way `hook_content` already is.

## ds status review listing

`summarize_change` (project-level) and `status_change` (per-change) gain a `Review` arm
that counts and lists reviews, the way steps are counted. Purely informational — reviews
never appear in phase or coverage tallies.

```rust
// duckspec/src/cmd/status.rs — inside the classify match
ArtifactKind::Review => reviews.push((file_path.clone(), relative.to_path_buf())),
```

## Current review in session orientation

This is the only cross-crate behavior change, and it lives entirely in duckboard, where
the orientation is already built. Three small edits:

```rust
// duckboard/src/data.rs
pub struct ChangeData {
    // … has_proposal, has_design, cap_tree, steps …
    pub reviews: Vec<String>,   // review filenames, sorted; loaded next to steps
}

// duckboard/src/area/change.rs
pub struct ChangeScopeFacts {
    // … phase, steps_done, step_count, active_step_tasks, next_command …
    pub current_review: Option<String>,   // highest-NN review filename, or None
}

pub fn change_scope_facts(name: &str, project: &ProjectData) -> Option<ChangeScopeFacts> {
    // … unchanged phase/next_command derivation …
    // current_review is computed once and set in EVERY return arm, so it
    // surfaces even pre-steps (a pre-implementation review under a proposal).
    let current_review = change.reviews.last().cloned();
    // … each `Some(ChangeScopeFacts { … current_review, … })`
}
```

```rust
// duckboard/src/scope.rs — render_change_orientation, appended when present
let review = match &facts.current_review {
    Some(r) => format!(" Current review: `reviews/{r}` (latest)."),
    None => String::new(),
};
// "… {progress}{next}{review} {authority}"
```

`SessionScope.change_facts` already carries `ChangeScopeFacts`, so no plumbing changes
between the facts builder and the hook.

## review capability and session/scope delta

The change ships its own contracts:

- `caps/review/spec.md` + `caps/review/doc.md` — a new top-level capability: reviews as
  advisory, sequentially-numbered `doc` artifacts; `ds create review` appends; current =
  highest `NN`; reviews never alter phase or next-stage.

- `caps/session/scope/spec.delta.md` + `doc.delta.md` — adds a requirement that a
  change-scope orientation surfaces the current review, and states that phase derivation
  ignores `reviews/`. Ships as a delta because `session/scope` already exists.

## Agent surface: schema, template, and commands

- `content/schemas/review.md` — the review document's guidance, what `ds schema review`
  prints. This is prose guidance (the conventional findings / severity /
  recommended-action / verdict shape), not a parser — exactly like
  `content/schemas/proposal.md` and `design.md`, which document doc-parser artifacts.
  `ds schema review` works the moment the file exists (`schema.rs` reads schemas by name);
  the `/ds-review` template references it the way `/ds-spec` references `ds schema spec`.

- `content/templates/review.md` — the `/ds-review` template, in the standard skeleton
  (`## Before write` … `## After write`), with two movements: produce
  `reviews/NN-<slug>.md`, then — only when the user is ready — generate review-sourced
  fix-steps whose `## Context` cites the review. `ds template
  review` works the moment the
  file exists (it reads templates by name).

- `content/commands/claude/ds-review.md` and `content/commands/opencode/ds-review.md` —
  the harness command/skill files. `ds init` needs **no code change**: `install_commands`
  globs every `.md` in the harness directory, so both are picked up automatically.

- `"review"` is added to `plan::STAGES` so `ds create hook review --pre/--post` works, for
  parity with other stages.

## Decisions

- **Reuse the doc schema and parser** — chosen over a dedicated review parser with
  structured findings. The stage is advisory-only; structure lives in the template, not
  the parser. Matches the proposal/design precedent (both are `doc`). Alternatives: a
  `parse::review` with severity/finding types (rejected: buys machine-readable findings
  the advisory model has already declined).

- **Orientation stays in duckboard** — `current_review` is added to
  `ChangeScopeFacts`/`scope.rs`, where phase and next-stage already live. Alternative:
  compute orientation in duckpond (rejected: relocating working logic for no gain).

- **Current review never gates** — `current_review` is set in every `change_scope_facts`
  arm but `next_command`/`phase` are untouched. Alternative: fold reviews into phase
  (rejected per proposal — advisory).

- **Append-only, no `--after`** — reviews are a chronological log; insertion and
  renumbering (which `create_step` supports) are intentionally omitted.

- **Thread the placeholder title** — rather than infer `# Review` from a filename that
  collides with steps' `NN-<slug>.md`, pass the title from the command that already knows
  the artifact kind.

- **Orientation cites the review filename, not its H1 title** — the orientation hands the
  agent an openable pointer (`reviews/02-post-implementation.md`), and the slug already
  carries the intent because it is derived from the title at create time. A title is not a
  path and would force the agent to locate the file. Alternative: read and render the H1
  (rejected: a file read for capitalization and spaces only).

- **`ds status` review listing is plumbing, not a specced behavior** — the listing ships
  as a deliverable (the proposal commits to it) but carries no `test: code` scenarios,
  matching the existing precedent where `ds status`'s step listing and `ds create step`'s
  numbering are unit-tested plumbing without capability specs. The `review` spec contracts
  the behavioral model — doc-artifact classification, sequential append-numbering, current
  = highest `NN`, and the advisory (never-gates) guarantee; the `ds status` listing, the
  `ds create review` wiring, the template, and the skill ride along as implementation
  step-tasks.

## Risks

- **Step/review filename-shape collision** → a shared, neutrally-named `NN-<slug>` helper
  (`extract_nn_slug` / `parse_nn_slug`) and an explicit placeholder title, so neither path
  guesses kind from the filename.

- **Archive validating reviews** → resolved: `classify` recognizes `Review` (so nothing
  reads as an unrecognized artifact), and the archive merge planner only handles `caps/`
  kinds — reviews ride along when the folder moves.

- **Pre-implementation review must still surface** → `current_review` is computed before
  the phase branches and set in all arms, so a review written under a proposal-only change
  still appears in the orientation.

## Open questions

None — both prior open questions (filename vs. H1 title in the orientation; whether the
`ds status` listing is specced) are resolved under Decisions.
