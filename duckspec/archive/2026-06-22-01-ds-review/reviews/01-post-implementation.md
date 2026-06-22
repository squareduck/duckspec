# Post-implementation review: advisory review stage

Reviewed the `ds-review` change against its proposal, design, and `review` /
`session/scope` specs, and against the full diff across duckpond, duckspec, and duckboard.
The change is correct, builds clean, and `ds audit` / `ds check` pass; implementation
fidelity to the design is high. No blockers. The findings are about a test gap on the
design's own headline risk and some reuse/consistency nits — none of which gate archival.

## Scope

The `caps/review` spec + doc, the `caps/session/scope` spec/doc deltas, all five steps,
and the implementation diff: `duckpond` (`layout.rs`, `plan.rs`, `check.rs`, `format.rs`,
`audit.rs`), `duckspec` (`cmd/create.rs`, `cmd/status.rs`, `cmd/check.rs`,
`cmd/archive.rs`, `cmd/index.rs`, content files), and `duckboard` (`data.rs`,
`area/change.rs`, `scope.rs`, and the three `ChangeData` literal sites). Read as a
post-implementation code review against the spec contract. Verification run: `cargo test`
(all green), `ds audit` (ok), `ds check` (ok).

## Findings

### The design's #1 risk is the one untested path — major

The design names the step/review filename collision as its top risk and resolves it by
threading an explicit `# Review` placeholder from the command instead of sniffing the
filename (`crates/duckspec/src/cmd/create.rs:89`). That mitigation lives entirely in the
CLI dispatch — `plan::create_review` does **not** decide the seed content. Yet the only
automated coverage is on the planner (`plan.rs` unit tests for numbering and slug
conflict); `cmd/create.rs` has zero tests, and `crates/duckspec/tests/` has no create-path
integration test at all.

So the exact line that defends against the headline risk — and the reviews-dir listing /
`create_review` wiring around it — could regress to emitting `# Step` (or to
mis-numbering) and every test would stay green. I hit a live instance of this gap during
the review: `ds create review` failed against the installed binary because nothing
exercises the subcommand end-to-end; it only worked after a local rebuild.

Recommend a single integration test under `crates/duckspec/tests/` that runs
`ds create review` twice in a temp project and asserts (a) the files land as
`01-…`/`02-…`, and (b) the seed content is `# Review`, not `# Step`. That locks in both
the numbering and the collision mitigation the design singled out.

### `build_reviews` re-rolls a looser NN parser instead of reusing duckpond's — minor

`crates/duckboard/src/data.rs:344` parses the `NN-` prefix with
`num_str.parse::<u32>().is_ok()`, accepting any digit count, while duckpond's canonical
`layout::extract_nn_slug` (`crates/duckpond/src/layout.rs:363`) — which this change
deliberately generalized to be shared — requires *exactly* two digits. duckboard already
depends on duckpond, so it could call the canonical helper. "Current review =
highest-numbered" is then computed not by parsing the number but by trusting
`read_sorted_dir`'s lexicographic order to equal numeric order (`change.reviews.last()` in
`area/change.rs:749`). For the canonical `01`–`99` names `create_review` emits, that is
correct; a hand-authored `1-x.md` (passes the loose filter, sorts after `02-y.md`) or a
100th review would make `.last()` report the wrong "current" review.

This faithfully mirrors the pre-existing `build_steps` right below it (`data.rs:363`), so
it is consistent with the codebase rather than new drift, and the trigger is unrealistic —
hence minor. Recommend reusing `layout::extract_nn_slug` for both `build_reviews` and
`build_steps` (parsing the number to pick the max rather than trusting sort order) as a
small follow-up, not a fix-step for this change.

### The `ds status` review listing and the whole CLI create path ship unspecced and untested — minor

The design's "Decisions" section explicitly classifies the `ds status` listing, the
`ds create review` wiring, the template, and the skill as plumbing that rides along
without `test: code` scenarios, citing the precedent that `ds status`'s step listing is
unit-tested-only. That is a defensible, stated call — flagging it only so the reader knows
the coverage boundary is intentional, not an oversight. The caveat: the cited precedent
("unit-tested plumbing") does not actually hold for the new code, since neither
`cmd/status.rs`'s `print_review_summary` nor `cmd/create.rs`'s review arm has *any* test.
The integration test in the major finding above covers most of this gap; no separate
action needed beyond it.

### Schema "summary required" may overstate what `ds check` enforces — question

`content/schemas/review.md` (the `ds schema review` output) states under Rules: "A summary
paragraph directly follows the H1." The spec's well-formed scenario also pairs an H1 with
a summary, but the only negative scenario specced and tested is *missing H1*
(`tests/review.rs:30`). If the shared document parser does not actually reject an H1-only
review with no summary, the schema guidance promises an enforcement that doesn't exist.
This is inherited from the proposal/design doc-schema siblings, so it is likely consistent
— but worth a one-line confirmation that "summary required" matches `parse::doc` behavior,
lest the schema train agents to expect a check that never fires. No fix implied if the
parser does enforce it.

## Verdict

Ready to archive. The change delivers exactly what the proposal and design promised, with
clean specs, passing tests, and a clean audit. The one finding worth acting on before (or
shortly after) archive is the **major** test gap: add an integration test for
`ds create review` so the design's headline collision-mitigation and the numbering
behavior can't silently regress. The remaining findings are advisory cleanups suitable for
a follow-up change, not blockers — and per the review contract, none of this gates
anything.
