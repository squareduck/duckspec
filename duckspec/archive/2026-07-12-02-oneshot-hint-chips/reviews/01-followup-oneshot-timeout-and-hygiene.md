# Followup: oneshot latency, model pick, hygiene

Post-implementation pass: confirm cheapest oneshot model, raise timeout, research live
model discovery for auto-cheapest, and clear clippy warnings.

## Scope

Implemented `oneshot-hint-chips` (product chips path). Inspected `duckchat` oneshot
budget/model selection (`worker`, `claude_code`, `grok`, `acp/runtime`) and current
`cargo clippy` / `cargo check` on duckboard + duckchat.

## Summary

```
| # | sev | lens | title | → next |
| --- | --- | --- | --- | --- |
| 1 | major | quality | Oneshot 10s budget too tight | /ds-step |
| 2 | minor | fidelity | Verify preferred haiku actually selected | /ds-step |
| 3 | minor | soundness | Live model list → auto-cheapest heuristic | /ds-propose |
| 4 | minor | quality | Clippy warnings after this change | /ds-step |
```

## Issues

### 1. Oneshot 10s budget too tight - quality/major

**Where:** `crates/duckchat/src/worker.rs:25` (`ONESHOT_CALL_BUDGET = 10s`)

**Why:** Reply hints often time out in practice; cold oneshot process + model call
commonly exceeds 10s, so users see empty chips after a long wait.

**Action:** Raise budget (e.g. 20–30s) and/or measure ensure_hot vs prompt separately;
keep timeout so hung agents still recover.

### 2. Verify preferred haiku actually selected - fidelity/minor

**Where:** `crates/duckchat/src/claude_code.rs:39` (`TITLE_MODEL = "haiku"`);
`crates/duckchat/src/acp/runtime.rs:291` (`pick_model`)

**Why:** Code prefers haiku only if the agent advertises that id on initialize; otherwise
first advertised model is used (may not be cheapest/fastest). Worth logging selected
oneshot model and confirming against real Claude agent init.

**Action:** Confirm runtime selection (log or test against real init models); if mismatch,
fix preference matching.

### 3. Live model list → auto-cheapest heuristic - soundness/minor

**Where:** `Provider::list_models`; Claude static list vs Grok ACP discovery; duckboard
`available_models()`

**Why:** Hard-coded preferred ids work but don’t adapt when catalogs change; no shared
“cheapest/fastest” ranking. Claude has no machine-readable CLI model list today (curated
aliases only); Grok already discovers from initialize.

**Action:** Separate slice (new change / propose): document feasibility; optional
`ModelInfo` tier + pick-oneshot helper. Not required to freeze chip UI.

### 4. Clippy warnings after this change - quality/minor

**Where:** `crates/duckboard/src/widget/text_edit.rs` unused `CONTENT_PAD_Y`;
`crates/duckboard/src/default_prompts.rs` `oneshot_under_input_chrome_visible` dead in
non-test builds

**Why:** Clean tree; warnings will fail stricter CI.

**Action:** Drop unused import; `#[cfg(test)]` the helper or fold tests onto
eligibility-only.

## Outcome

Product path works. Before archive: tighten oneshot reliability (budget + model
verification) and clear warnings. Model-discovery auto-pick is research-backed follow-on,
not required to freeze chip UI.
