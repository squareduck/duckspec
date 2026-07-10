# Post-implementation chrome ladder

Post-implementation review of the review-aware obvious-chrome ladder and Create change
affirm. Thinking is sound and the code matches the caps; one product surface still
contradicts the new orientation rule.

## Scope

Stage: **post-implementation** (proposal, design, both cap deltas, all three steps
complete, `ds audit` clean).

Examined:

- `proposal.md`, `design.md`

- `caps/chat/obvious-bubble/{spec,doc}.delta.md`

- `caps/session/scope/{spec,doc}.delta.md`

- `crates/duckboard/src/obvious_bubble.rs`, `crates/duckboard/src/area/change.rs`
  (`change_scope_facts`, `build_obvious_chrome`, composition tests)

- Soft-hint path in `crates/duckboard/src/main.rs` (lifecycle[0] only)

- `crates/duckspec/content/templates/review.md` handoff (adjacent product surface)

## Findings

### Review template still denies orientation impact — fidelity/major

`crates/duckspec/content/templates/review.md` Handoff still states:

> A review never changes the orientation's suggested next stage on its own.

This change deliberately makes the opposite true: presence of any review file steers
`change_scope_facts` (and therefore orientation `next_command` and chrome) onto the rework
ladder. After archive, every `/ds-review` session will tell the agent a rule that the
living `session/scope` cap and duckboard contradict.

**Why it matters:** Agents treat the template as operational law. A false handoff note is
durable confusion on every review, not a one-off doc nit.

**Recommended action:** Update that handoff line (and any sibling wording) so it matches
the review-aware ladder — e.g. that creating a review may change chrome/orientation next
stage to the rework options, with archive remaining available as a ranked option when
there are no open steps. No cap change required if the living `session/scope` delta is
already correct; this is template/code alignment only.

### Affirm comments still say Confirm/Commit only — quality/minor

A few call-site comments still describe the enter chip as Confirm/Commit only:

- `crates/duckboard/src/theme.rs` (`chat_obvious_chip_enter`)
- `crates/duckboard/src/widget/agent_chat.rs` (`ObviousChipTone::Enter`)

`Affirm` and `affirm_chip_label` docs were updated; these were not. Low lasting cost, but
they mislead the next reader of the render path.

**Recommended action:** Mention Create change alongside Confirm/Commit in those two
comments.

## What went well

- Single ladder in `change_scope_facts` shared by chrome, soft-hint first option, and
  orientation — no dual source of truth.

- Priority order is easy to read and matches the frozen table (open steps before review
  rework before all-done before pre-step).

- Gate narrowing is the right product call: Confirm is freeform early-stage, not apply.

- Coarse “any review file” signal with archive as third option is an honest trade-off vs
  parsing verdicts; documented and tested.

- Scenario coverage for the new arms is tight and linked; 293 duckboard tests pass.

## Verdict

**Not quite archive-ready until the review-template handoff is fixed.** The capability
chain and implementation are faithful and well-made for a pure composition change. Leaving
the `/ds-review` template asserting the old “reviews never affect next stage” rule would
ship a self-contradicting product surface on the exact stage this change rewires. Fix that
(and optionally the two comment nits), then accept.
