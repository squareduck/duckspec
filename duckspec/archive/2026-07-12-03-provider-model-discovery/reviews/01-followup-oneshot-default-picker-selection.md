# Oneshot default picker selection

User-led followup after implementation: feature works, but Settings oneshot pickers show
the first catalog model as the default instead of the string-match cheap/fast default.

## Scope

Post-implementation Settings Chat oneshot pickers (`agent_input_hints` on) for Claude Code
and Grok, with live catalog models. Compared picker selection in
`crates/duckboard/src/area/settings.rs` to `agent::resolve_oneshot_model`.

## Summary

```
| # | sev | lens | title | → next |
| --- | --- | --- | --- | --- |
| 1 | major | fidelity | Oneshot pickers ignore string-match default when config is empty | /ds-step |
```

## Issues

### 1. Oneshot pickers ignore string-match default when config is empty - fidelity/major

**Where:** `crates/duckboard/src/area/settings.rs` (oneshot pickers under Chat); selection
uses `selected_model_choice` with only the configured id, so unset falls through to the
first catalog entry. Runtime path: `agent::resolve_oneshot_model` /
`resolved_oneshot_model_for`.

**Why:** With no stored preference, the UI shows first-catalog defaults (e.g. Claude
Sonnet 5, Grok 4.5) while the intended and worker-side ladder prefers string-match
cheap/fast models (haiku / composer-fast). Pickers misrepresent what oneshots will use and
invite accidental “save” of the wrong model.

**Action:** When rendering oneshot pickers, select the model from the same resolution
ladder as runtime (`resolve_oneshot_model`: configured if in catalog → string-match
default → first). Optionally add a scenario that unset config shows the string-match
default; plan via `/ds-step` (and `/ds-spec` only if a new scenario is wanted).

## Outcome

One major fidelity issue; change is otherwise usable. Next: plan a small fix step to align
Settings selection with `resolve_oneshot_model`. Not archive-ready until that mismatch is
fixed or explicitly accepted.
