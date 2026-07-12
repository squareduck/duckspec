# Global default model

Give duckboard a concrete global main-chat default model, treat the project default as an
override, and make “available” mean live catalog contents—so missing models surface
clearly instead of silent fallbacks.

## Motivation

Main-chat defaults only exist per project today, and clearing them falls through to a
hardcoded built-in model. There is no user-facing global default, so machines without that
backend still behave as if a real choice was made. Project “No default” reads as optional
policy when it is really “use the compile-time floor.” Oneshot pickers and the process
catalog also blur availability: harness rows are driven from a fixed list, and empty
rediscovery can keep a last-good model list after a provider is no longer usable.

Why now: multi-harness selection and catalog discovery are already in place; fixing the
default cascade and availability story before more settings pile on avoids baking in the
hardcoded floor and stale-catalog behavior.

## Intent

- A global default main-chat model is a concrete harness-tagged choice the user can set in
  Settings, always visible as a global setting

- The project default is an optional override of that global choice; clearing it means
  “use global,” not “let the CLI pick”

- Effective model resolution prefers per-chat pin, then project override, then global
  default

- When the preferred model is not in the process catalog, the chat model selector shows
  **Missing** and **send is blocked**—the app does not invent a substitute model

- Settings separates global configuration from this-project overrides

- Oneshot model selectors appear only for harnesses that currently have models in the
  process catalog

- Empty or failed model rediscovery clears that harness’s catalog slice (no
  keep-last-good); successful non-empty discovery still replaces the slice

- Existing per-project defaults migrate as overrides; a first-run global default is seeded
  from the former built-in when still present in the catalog, otherwise from an available
  catalog model

## Non-goals

- Per-harness global defaults for main chat (oneshot stays per-harness; main chat stays
  one model)

- Auto-picking a fallback model when the preferred choice is missing

- A separate provider “installed / on PATH / healthy” API beyond catalog-driven
  availability

- Changing oneshot resolution among catalog models (configured → string-match default →
  first)

- Changing how per-chat pins work beyond participating in the cascade

- Install or upsell UI for harnesses that are not available
