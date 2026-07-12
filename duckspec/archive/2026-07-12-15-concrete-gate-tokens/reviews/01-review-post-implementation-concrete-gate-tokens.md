# Post-implementation review: concrete gate tokens

Stock style, templates, and codex match the proposal: decision-named gates, restored
separate `/ds-spec` map, REMOVE kept. Two minor doc/fidelity nits; ready to freeze after
optional polish.

## Scope

Proposal, design, four steps, and the implementation under
`crates/duckspec/content/schemas/style.md`, `crates/duckspec/content/templates/*`, and
`duckspec/codex/template-and-schema-authoring.md`. No caps/code layer. Post-implementation
content review.

## Summary

```
| # | sev | lens | title | → next |
| --- | --- | --- | --- | --- |
| 1 | minor | fidelity | meta-cards doc still demos bare `confirm` | /ds-step |
| 2 | minor | fidelity | archive `commit` chip still carries a reason | /ds-step |
```

## Findings

### 1. meta-cards doc still demos bare `confirm` - fidelity/minor

**Where:** `duckspec/caps/chat/meta-cards/doc.md` (example card + “for example `confirm`”
prose)

**Why:** Living cap docs re-teach bare `confirm` / `reject` with reasons next to the new
stock rule, so agents and humans reading caps can relearn the anti-pattern.

**Action:** Update examples to a decision-named token (and a slash line with a reason if
demonstrating optional reasons).

### 2. archive `commit` chip still carries a reason - fidelity/minor

**Where:** `crates/duckspec/content/templates/archive.md` handoff (`commit` + reason)

**Why:** Reason-split says decision tokens stand alone; `` `commit`  commit
change `` is
redundant noise after the new rule.

**Action:** Emit `` `commit` `` alone (or a more specific token if desired).

## Verdict

Well-conceived and well-made for a stock-content change: vocabulary is consistent across
style, templates, and codex; spec map restore matches design; no parser/runtime scope
creep. The two findings are low-cost polish, not blockers — archive as-is is defensible; a
short step pass would align residual examples with the new rule.
