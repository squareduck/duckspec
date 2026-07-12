# Align oneshot picker with resolve ladder

Settings oneshot pickers select the resolved oneshot model (config → string-match →
first), not the first catalog entry when preference is unset — matching
`agent::resolve_oneshot_model` and the worker path.

## Prerequisites

- [x] @step oneshot-settings-pickers

## Context

Followup `reviews/01-followup-oneshot-default-picker-selection.md`: with agent input hints
on and no stored preference, Claude showed first live catalog model (e.g. Sonnet 5) and
Grok showed Grok 4.5 instead of haiku / composer-fast string-match defaults.

## Tasks

- [x] 1. In `crates/duckboard/src/area/settings.rs` oneshot pickers, select via
         `agent::resolve_oneshot_model` (or `resolved_oneshot_model_for`) using configured
         id + catalog models, then map that id to a `ModelChoice` (do not use bare first
         catalog entry when config is absent)

- [x] 2. Unit test: empty/unset config with a catalog that includes a string-match default
         (e.g. haiku / composer-fast) yields that model as the selected oneshot choice,
         not the first catalog entry

- [x] 3. Confirm Claude Code and Grok picker selection both use the same resolve path
         (shared helper if it keeps the logic in one place)
