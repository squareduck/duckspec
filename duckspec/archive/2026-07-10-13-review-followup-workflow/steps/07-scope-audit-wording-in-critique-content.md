# Scope audit wording in critique content

Retarget bare `ds audit` in review/followup content so change well-formedness means scoped
audit (or no audit for followup), matching apply vs archive.

## Context

Addresses the issue in `reviews/04-followup-audit-scope-in-critique-templates.md`. Content
only — no product-code change.

- Bare `ds audit` = whole-project health (`templates/archive.md`).

- `ds audit <change>` = change progress / well-formedness for a change
  (`templates/apply.md`).

- Followup must not instruct or imply running audit; only `ds format` / `ds check` on the
  followup file.

## Tasks

- [x] 1. In `crates/duckspec/content/templates/review.md`, reword Role and “don’t
         re-verify” guidance so any command for change well-formedness is
         `ds audit <change>` (or “static tooling” without bare `ds audit`); do not
         instruct a project-wide audit mid-review

- [x] 2. In `crates/duckspec/content/schemas/review.md`, reword well-formedness and “don’t
         re-verify” lines the same way — prefer `ds audit <change>` / `ds check` when
         naming CLI forms

- [x] 3. In `crates/duckspec/content/schemas/followup.md`, remove or reword bare
         `ds audit` so followup does not imply running audit; static well-formedness may
         mention `ds check` only, or conceptual tooling without a bare `ds audit` command

- [x] 4. Confirm `crates/duckspec/content/templates/followup.md` has no audit run step
         (format + check on the followup file only); leave as-is if clean

- [x] 5. Spot-check `templates/apply.md` still says `ds audit <change>` and
         `templates/archive.md` still says bare `ds audit` for whole-project; do not
         change those unless a copy-paste error appears
