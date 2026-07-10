# Review and followup workflow — Design

Kind-prefixed critique history under one `reviews/` log, peer review/followup schemas and
templates with a scannable body, and lifecycle chrome that keeps both critique modes
available mid-steps and after completion.

## Approach

Three layers cooperate; nothing new is invented at the layout/classify layer.

```
  ┌─────────────┐                 ┌──────────────┐
  │ /ds-review  │                 │ /ds-followup │
  │ agent scan  │                 │ user-led     │
  └──────┬──────┘                 └──────┬───────┘
         │  ds create review             │  ds create followup
         │  template + schema review     │  template + schema followup
         ▼                               ▼
              changes/<name>/reviews/
              ├── 01-review-post-implementation.md
              ├── 02-followup-collapse-policy.md
              └── 03-review-second-pass.md
                         │
                         │  ArtifactKind::Review (unchanged)
                         │  document schema validation only
                         │
                         │  document-first write gate
                         │  (recommend next stage; no in-stage plan edit)
                         ▼
              /ds-spec · /ds-step · /ds-apply · explicit post-doc fix
              (plan/code outside critique spine)

  disk state ──► change_scope_facts ──► lifecycle_commands
                         │
                         ├── obvious chrome chips
                         └── session orientation next_command = lifecycle[0]
```

**Strategy**

1. **Storage stays one log.** Any `.md` under `reviews/` remains `ArtifactKind::Review`
   and validates as a document. Kind is a create-time filename convention (`review-` /
   `followup-` prefix on the slug), not a new kind or directory.

2. **Create is kind-aware.** Plan and CLI take an explicit kind; the human title is
   slugified and the kind prefix is prepended once. Numbering and slug uniqueness stay
   append-only as today.

3. **Presentation is content.** Schemas recommend a summary table + structured detail
   headings; templates enforce that shape in agent workflow. `ds check` does not
   hard-validate tables.

4. **Chrome stops treating history as terminal.** Open steps always offer apply + both
   critique modes. After steps (or with reviews and no open work), critique remains
   listed; rework (step/spec) and archive stay available where they already matter.

```
  lifecycle arms (bare names → chrome adds /)

  open steps:
    apply, review, followup

  all done, no reviews yet:
    archive, review, followup

  no open steps + has reviews:
    step, spec, review, followup, archive

  pre-step arms (proposal / design / caps / empty):
    unchanged — no critique chips
```

`has_review` still drives Confirm/Reject gate eligibility and phase labels; it no longer
removes `/ds-review` (or `/ds-followup`) from the lifecycle list.

## Kind-prefixed create

Centralize kind in duckpond plan so CLI, caps, and tests share one rule.

```
  title "Post-implementation soundness"
       │
       ▼
  slugify → "post-implementation-soundness"
       │
       ▼
  CritiqueKind::Review → full_slug = "review-post-implementation-soundness"
       │
       ▼
  next NN → changes/<c>/reviews/03-review-post-implementation-soundness.md
```

```rust
// crates/duckpond/src/plan.rs

pub enum CritiqueKind {
    Review,
    Followup,
}

impl CritiqueKind {
    pub fn as_str(self) -> &'static str { /* "review" | "followup" */ }
}

/// Plan creation of a critique file under reviews/.
/// `name` is the human title (slugified); kind prefix is applied here, not by
/// the caller embedding "review" in the title.
pub fn create_critique(
    name: &str,
    kind: CritiqueKind,
    change: &str,
    active_changes: &[String],
    existing_reviews: &[String],
) -> Result<Plan, PlanError> {
    // slug = format!("{kind}-{}", slugify(name))
    // reject empty slugify(name); reject slug uniqueness on full slug
    // next_nn = highest existing NN + 1
    // creates: reviews/{nn:02}-{slug}.md
    todo!()
}

// Keep a thin wrapper or replace call sites:
pub fn create_review(
    name: &str,
    change: &str,
    active_changes: &[String],
    existing_reviews: &[String],
) -> Result<Plan, PlanError> {
    create_critique(name, CritiqueKind::Review, change, active_changes, existing_reviews)
}

pub const STAGES: &[&str] = &[
    "explore", "backfill", "propose", "design", "spec", "step", "apply",
    "archive", "verify", "review", "followup", "codex",
];
```

CLI:

```rust
// crates/duckspec/src/cmd/create.rs

CreateCommand::Review { name, change } => { /* create_critique(..., Review) */ }
CreateCommand::Followup { name, change } => { /* create_critique(..., Followup) */ }

// forced placeholder H1:
//   Review   → "# Review\n"
//   Followup → "# Followup\n"
```

**Legacy files.** Create always writes kind-prefixed paths. Existing
`01-post-implementation.md` files remain classifiable and valid; they are not renamed.
Specs document that recognition does not require a kind prefix; only new creates enforce
it.

**Slug uniqueness.** Uniqueness is on the full slug (`review-foo` vs `followup-foo` may
both exist). Templates instruct agents not to put the kind word in the title so plan does
not produce `review-review-…`.

## Scannable schemas and templates

Peer content files under `crates/duckspec/content/`:

```
schemas/
  review.md      ← rewrite: Summary table + structured Findings
  followup.md    ← new: same spine, followup labels (Issues / Outcome)
templates/
  review.md      ← rewrite: scannable write + document-first write gate
  followup.md    ← new: explore voice, same write gate spine
commands/
  claude/ds-followup.md
  opencode/ds-followup.md
```

Both schemas keep the stock schema section shape (intro, Structure, Rules / Severity or
equivalent, Quality, Formatting, Example). Body contract:

```markdown
# <Title>

<summary: stage + headline>

## Scope
…

## Summary

| # | sev | lens | title | → next |
|---|-----|------|-------|--------|
| 1 | …   | …    | …     | …      |

## Findings          # followup may title this ## Issues
### 1. <title> — <lens>/<severity>
**Where:** …
**Why:** …
**Action:** …        # recommended next stage — not work already done

## Open questions    # optional
## Verdict           # followup: ## Outcome
```

Templates keep the stock template section shape (Before write / Role / Voice / Context /
Instructions / Write gate / Handoff / After write).

**Shared write-gate spine (both templates):**

```
1. Critique / co-decide issues (mode differs by voice)
2. ds create {review|followup} "<title>" --in <change>
3. Write scannable body per schema; ds format + ds check
4. Present chat triage (same table as file Summary) + handoff
5. Do not edit proposal / design / caps / steps / product code in this stage
6. Plan or code fixes only if the user explicitly asks after the document exists
   (out of band), or via a later stage (/ds-spec, /ds-step, /ds-apply)
```

**Voice split**

```
| | review | followup |
|--|--------|----------|
| Role | strict senior critique | discovery partner (explore-like) |
| Issue source | agent reads chain + code | conversation with user |
| Schema | `ds schema review` | `ds schema followup` |
| Create | `ds create review` | `ds create followup` |
```

**Downstream template nits (same change, small):** `step` / `apply` / `archive` handoffs
and “latest review” wording already mean highest-numbered file under `reviews/` — that
continues to cover both kinds. Reply-suggest lifecycle string list in
`crates/duckchat/src/reply_suggest.rs` gains `/ds-followup`.

`ds template followup` works by dropping `followup.md` into the templates dir (no code
change beyond the stock-template hook test discovering the new file).

## Lifecycle dual-critique

Single pure function remains the source of truth:

```rust
// crates/duckboard/src/area/change.rs — change_scope_facts

// open steps — always critique peers of apply
if open {
    return Some(scope_facts(
        "implementing steps",
        …,
        &["ds-apply", "ds-review", "ds-followup"],
        current_review,
    ));
}

// no open steps + history — rework + re-critique + archive
if has_review {
    return Some(scope_facts(
        /* all_done ? "all steps complete, review on file"
                     : "review on file, no open steps" */,
        …,
        &["ds-step", "ds-spec", "ds-review", "ds-followup", "ds-archive"],
        current_review,
    ));
}

// all done, no history yet
if all_done {
    return Some(scope_facts(
        "all steps complete",
        …,
        &["ds-archive", "ds-review", "ds-followup"],
        current_review,
    ));
}

// pre-step arms unchanged
```

```
  before                         after
  ─────                         ─────
  open + review → [apply]       open → [apply, review, followup]
  open + ∅      → [apply, review]
  done + ∅      → [archive, review]   → [archive, review, followup]
  ∅ open + review → [step, spec, archive]
                              → [step, spec, review, followup, archive]
```

`next_command` stays `lifecycle_commands[0]` so orientation and oneshot soft hint remain
apply-first mid-steps and archive- or step-first after, without blocking access to
critique chips (⌘2 / ⌘3).

**Caps in lockstep**

- `chat/obvious-bubble` — rewrite the ordered-arm list and scenarios that lock “apply only
  when review on file” and “archive then review” without followup.

- `session/scope` — lifecycle reflection scenarios that assert next-stage `step` when
  all-done + review must match the new first option (`ds-step` on the rework arm still
  holds). Doc table updated to the arms above.

**Gate row.** Unchanged rule: nonempty session + (has review | no steps | archive in
lifecycle) still shows Confirm/Reject. Open steps with only lifecycle chips (no review
yet) stay lifecycle-only; once a critique file exists mid-impl, gate appears as today.

## Capability deltas

```
caps/
├── review/              delta: kind-prefixed create; dual modes in doc;
│                        legacy unprefixed still recognized
├── chat/obvious-bubble/ delta: lifecycle arms + scenarios
└── session/scope/       delta: orientation/lifecycle table + scenarios
```

No new capability path. Followup is a create kind + content surface on the existing review
recognition story.

**review** — extend Sequential numbering / Filename slug requirements so create for review
and followup kinds produces `NN-review-<slug>` and `NN-followup-<slug>`; document dual
purpose (agent vs user-led) and that body structure is conventional. Recognition
requirement stays path-based (any reviews `.md`).

**chat/obvious-bubble** — replace the four review-sensitive arms with the three arms in
Approach; add scenarios for open-steps with existing reviews still listing
review+followup, and all-done listing followup.

**session/scope** — keep “first lifecycle option = next stage”; update doc ladder table;
adjust scenarios only where expected command lists change.

## Decisions

- **Kind via create enum, not title text** — plan prepends `review-` / `followup-` after
  slugify. Alternatives: parse kind from title (fragile); separate `followups/` directory
  (rejected in proposal).

- **Same `ArtifactKind::Review`** — no classifier fork. Alternatives: new `Followup` kind
  (extra layout/check/status surface for no validation gain).

- **Soft scannable body** — schema + template only; `ds check` stays document rules.
  Alternatives: custom review parser (out of scope / proposal).

- **Rework arm order `step, spec, review, followup, archive`** — primary after findings is
  turn-into-work; re-critique and archive remain reachable. Alternatives: critique-first
  when history exists (deprioritizes the common “act on findings” path).

- **Pre-step arms without critique chips** — proposal scoped mid-steps and completion;
  pre-impl review remains possible via explicit `/ds-review` type-in without chrome
  promotion.

- **`create_critique` + CLI Followup subcommand** — mirrors step/review symmetry.
  Alternatives: `ds create review --kind followup` (fine, slightly worse discoverability
  for agents).

## Risks

- **Chrome clutter (3–5 chips)** → keep primary action first (`apply` / `archive` /
  `step`); critique is secondary. Caps lock exact order in tests.

- **Double kind prefix in slug** → templates: title without kind word; plan always
  prefixes exactly once from `CritiqueKind`.

- **Critique scope creep into plan or code** → document-first write gate: only the
  critique file is written in-stage; plan/code only on explicit post-doc request or later
  stages.

- **Orientation “current review” is highest NN of either kind** → acceptable: latest
  critique record is the right soft pointer; step template already uses highest-numbered
  file.

- **Test churn on lifecycle** → obvious-bubble and change.rs unit tests encode old arms;
  update in the same step as the function change.

## Open questions

None. Chip order, document-first critique write gate, mid-step availability, and
single-change scope are fixed by the proposal and this design.
