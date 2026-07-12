# Honest unset global default

Re-seed the global default when it is cleared but the catalog has models, and show
**Missing** on the Settings global picker when global is unset or not in the catalog.

## Context

Review `reviews/02-review-post-implementation-review-global-default-model.md`:

1. `ResetDefaults` clears `default_model` and seed only runs once on `ModelCatalogReady`.

2. `selected_model_choice(None)` makes the global picker look like the first catalog model
   is selected when config is unset.

## Tasks

- [x] 1. After `ResetDefaults` in `crates/duckboard/src/area/settings.rs` (and any clear
         of global default), call `agent::seed_global_default_if_unset` with the process
         catalog and save when a seed is written

- [x] 2. Optionally also re-seed when stamping session defaults / on Settings open if
         `default_model` is `None` and catalog is non-empty — so any clear path heals
         without relying only on Reset

- [x] 3. In `global_model_section`, when global is unset or not present in the process
         catalog, use `missing_closed_model_choice` (or equivalent) so the closed control
         shows **Missing** — do not use `selected_model_choice(None)` on a catalog-only
         list

- [x] 4. When global is set and in the catalog, keep showing the normal catalog selection
         via `selected_model_choice`
