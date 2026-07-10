# Post-implementation — orientation paths and handoffs

Reviewed the change end-to-end after all three steps. Path and next-stage code match the
contract; the handoff matrix is mostly right but leaves a hole on the happy path after
review, and the archive commit wording is project-specific in a shipped template.

## Scope

Full chain: proposal, design, `session/scope` deltas, steps 01–03, and the implementation
under `crates/duckboard/src/scope.rs`, `crates/duckboard/src/area/change.rs`, and
`crates/duckspec/content/templates/*.md`. Post-implementation; `ds audit` on the change is
clean.

## Findings

### Archive is never ranked after the user takes review — soundness/major

Intended lifecycle after implementation is apply-done → review → archive → commit. Apply's
handoff ranks ① `/ds-review` ② `/ds-archive`, but if the user takes ①, the next ranked
surface is the **review** handoff, which only offers `/ds-spec` or `/ds-step` for findings
(or nothing when the review is the whole value). Orientation's single `next_command` for
all-steps-complete is permanently `ds-review` (`crates/duckboard/src/area/change.rs`
all-done branch), so later sessions keep advertising review, never archive.

Net: archive is only suggested as apply's secondary at the moment steps finish. A normal
"review first" path never ranks `/ds-archive` again. Agents must invent it.

**Action:** When the change is all-steps-complete, review handoff should rank archive once
findings are addressed or the verdict is accept/ready (e.g. ① `/ds-archive` with no open
findings; keep ① `/ds-spec`/`/ds-step` when findings need work). Optionally also teach
orientation a post-review primary — but fixing the review handoff is enough to close the
loop without multi-value `next_command`.

### Archive commit handoff hardcodes this project's VCS — quality/major

`crates/duckspec/content/templates/archive.md` Handoff says to propose a commit "in the
project's usual form (read `AGENTS.md` when present; **this repo uses**
`type(optional-scope): …` via jj)". These templates ship with duckspec for every consumer
project. Baking in jj and this repo's commit grammar will mis-instruct agents on git-only
or differently convented trees.

**Action:** Keep "propose a message from project conventions / `AGENTS.md` when present;
wait for confirmation; never auto-commit." Drop the "this repo uses jj" clause from the
shipped template.

### Proposal Impact matrix omits design open-questions branch — fidelity/minor

Proposal Impact still lists design as ① `/ds-spec` ② `/ds-step` only. Design and templates
correctly make open questions the design primary when any remain. Historical pitch drift,
not a code defect — update the proposal table if you want the change folder
self-consistent, or leave it as superseded by design.

## Verdict

Acceptable core: project-root paths, all-done → review in facts/orientation, and the ≤2
handoff rewrite are faithful to the design and well-sized. I would not archive yet: the
review→archive happy path does not rank archive anywhere after the user follows the new
primary, and the archive template's commit wording is wrong for a universal template. Fix
those two majors before accepting; the proposal-table nit is optional polish.

## What went well

- Small surface: string paths + one ladder branch + content handoffs — no new modules.
- Spec delta is economical (retarget + path THENs, not a parallel path requirement).
- Tests track the new contract without ceremony.
