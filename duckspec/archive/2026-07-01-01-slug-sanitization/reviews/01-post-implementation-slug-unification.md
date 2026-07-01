# Post-implementation slug unification

Reviewed the `slug-sanitization` change end-to-end — proposal, design, the new `slug` cap
and `review` deltas, and the code across `duckpond` and `duckboard`. The change is sound,
faithful, and cleanly made; two minor cleanups would leave it tidier, but nothing blocks
acceptance.

## Scope

The proposal and design, the `caps/slug` spec/doc and `caps/review` deltas, all three
steps, and the diff across
`crates/duckpond/src/{slug.rs,plan.rs,parse/step.rs,artifact/step.rs,lib.rs}` and
`crates/duckboard/src/idea_store.rs`. Post-implementation: the full chain down to code.
`cargo test -p duckpond -p duckboard` passes; all callers of the three deleted `slugify`
copies were confirmed repointed with no dangling references.

## Findings

### `review` doc re-describes the slug rule inline — quality/minor

`caps/review/doc.delta.md:7-8` names "the canonical slug rule" (good) but then glosses it
— "the title lowercased, with each run of non-alphanumeric characters mapped to a single
`-`". That gloss is a partial restatement of the `slug` cap: it omits the
Unicode-preservation and trim steps, and it is a second place that goes stale if the rule
ever changes. This change exists to collapse a duplicated rule into one source of truth;
re-stating it in a neighbouring doc reintroduces the same drift surface at the cap layer.
The spec delta gets this right (`spec.delta.md:5` references the rule without restating
it). Drop the gloss from the doc and let the reference to the `slug` capability carry it.

### The headline step-punctuation regression has no direct test — fidelity/minor

The proposal frames punctuated *step* titles as the sharp bug — creation and validation
slugs disagreed, so the file "immediately fails `ds check` with a slug mismatch"
(`proposal.md:28-30`). The added tests cover review punctuation normalization (`plan.rs`
`review_punctuated_title_is_dash_normalized`) and empty-slug rejection on both paths, but
no test exercises a punctuated *step* title round-trip — that the created step filename
matches the slug the parser derives from its H1. The fix is correct by construction (both
sides now call `crate::slug::slugify`) and transitively covered by the review test, so
this is minor, but the case the change was written to kill deserves its own regression
guard. Add a `create_step` test mirroring the review one, asserting a punctuated title
produces the dash-normalized filename.

## Verdict

Ready to accept. The proposal diagnoses a real, visible defect and two latent
inconsistencies; the design justifies its three load-bearing decisions (one bucket,
policy-free `slugify`, dedicated module) and correctly keeps the shared function a
byte-for-byte relocation of the proven step rule, so no existing step slug moves. The code
realizes it faithfully: three copies collapse to one, every caller is repointed,
empty-slug policy sits at each boundary as designed, and all five `slug` scenarios plus
both `review` scenarios are tested. The `idea_slug` helper is a small, sensible divergence
from the design's inline sketch — it reads better than an inline fallback, not worse.
Neither finding is structural: fold the doc gloss into a reference and add the step
round-trip test, and this is done.
