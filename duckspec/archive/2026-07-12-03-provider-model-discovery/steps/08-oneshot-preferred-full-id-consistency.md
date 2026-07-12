# Oneshot preferred full-id consistency

Stop bare-alias oneshot preference when advertise uses full API ids; host-resolved catalog
ids only, and align exact/string match so cheap defaults actually win.

## Prerequisites

- [x] @step align-oneshot-picker-with-resolve-ladder

## Context

Review `reviews/02-review-provider-model-discovery-post-implementation.md` finding 1:
`pick_oneshot_model` requires exact id match; live Claude advertise uses full ids;
`open_oneshot_runtime` falls back to bare `TITLE_MODEL` (`haiku` /
`grok-composer-2.5-fast`) when host preferred is `None`, so oneshots can land on the first
advertised model (often Sonnet) instead of a cheap/fast match.

## Tasks

- [x] 1. Remove bare-alias `preferred.or_else(TITLE_MODEL)` from Claude and Grok
         `open_oneshot_runtime`; pass the host `Option` through unchanged

- [x] 2. Fix transitional `title_summary` / `reply_suggestions` so they do not hardcode
         bare `TITLE_MODEL` when live advertise uses full ids (use catalog-aware resolve
         or the same preferred the worker would get)

- [x] 3. Align `pick_oneshot_model` with host string-match needles **or** guarantee
         `spawn_worker` always receives a full catalog id from `resolve_oneshot_model`
         when the catalog is non-empty

- [x] 4. When the process catalog fills after subscription start, ensure the agent
         subscription re-resolves oneshot preferred (identity already includes
         `oneshot_model` — wake UI or re-resolve so the worker is not stuck with
         `None`/alias)

- [x] 5. Unit tests: preferred bare `haiku` against full-id advertise still selects a
         haiku id (if needle path kept); when catalog is non-empty, host resolve yields a
         catalog id that exact-matches an advertised model

## Outcomes

- `pick_oneshot_model` now: exact → substring preferred → haiku/composer-fast/fast needles
  → first. Bare `haiku` matches full API ids even if the worker still has preferred `None`
  or a bare needle.

- Catalog-fill UI wake remains step 09; oneshot correctness no longer depends on exact
  host id alone.
