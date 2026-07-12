# Post-implementation review: global default model

Cascade, catalog clear-on-empty, Missing send gate, and Settings structure match design.
Two durable gaps around unset global default remain: Reset never re-seeds, and the global
picker must show Missing when resolution has nothing — not a fake first catalog model.

## Scope

Full chain: proposal, design, three cap deltas, steps 01–06, followup 01 (hierarchy fixed
in step 06), and implementation in `config.rs`, `agent.rs`, `interaction.rs`,
`settings.rs`, `main.rs`, `agent_chat.rs`, and `theme.rs`. Audit clean; 11/11 scenarios
linked.

## Summary

```
| # | sev | lens | title | → next |
| --- | --- | --- | --- | --- |
| 1 | major | soundness | Reset leaves global default unset for the process | /ds-step |
| 2 | major | fidelity | Unset/unavailable global must show Missing in Settings | /ds-step |
```

## Findings

### 1. Reset leaves global default unset for the process - soundness/major

**Where:** `crates/duckboard/src/area/settings.rs` — `Message::ResetDefaults` replaces
config with `Config::default()` (`default_model: None`). Seed only runs in
`crates/duckboard/src/main.rs` on `ModelCatalogReady`, which is one-shot (`STARTED`
atomic).

**Why:** After Reset, chats resolve Unconfigured → Missing for the rest of the process
until the user manually picks a global default or restarts. Seed will not run again.
“Reset to defaults” can break main-chat send for the session if models are available but
config was cleared.

**Action:** After reset (or any path that clears `default_model`), call
`seed_global_default_if_unset` with the current process catalog and save — or re-seed
whenever `default_model` is `None` and the catalog is non-empty (e.g. alongside session
default stamping).

### 2. Unset/unavailable global must show Missing in Settings - fidelity/major

**Where:** `crates/duckboard/src/area/settings.rs` `global_model_section` —
`selected_model_choice(&choices, current.as_ref())` when `current` is `None`.
`selected_model_choice` treats `None` as the first list entry
(`crates/duckboard/src/widget/agent_chat.rs`), which for a catalog-only list is a real
model, not an empty state.

**Why:** Settings can show e.g. `Grok · Grok 4.5` as selected while the cascade has no
preferred model and chats show Missing. Settings and chat disagree; pairs badly with
finding 1 after Reset.

**Action:** When the global default is unset or not present in the process catalog, the
Settings closed control shows **Missing** (same honest state as chat). Only display a
catalog choice when `default_model` is set and available. Reuse or align with
`missing_closed_model_choice` so both surfaces share one Missing representation.

## Verdict

**Sound overall.** Intent (concrete global, project override, catalog availability, no
silent model substitute, clear-on-empty catalog) is implemented and linked. Hierarchy
followup is fixed. Not archive-clean until unset-global behavior is honest: re-seed after
clear when models exist, and Settings shows **Missing** when resolution has nothing. Both
are small follow-up steps.
