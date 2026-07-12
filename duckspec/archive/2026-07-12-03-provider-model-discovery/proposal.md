# Provider model discovery

Selectable models (and their context windows) come from each provider’s live catalog, with
human-readable labels and global oneshot model choices per provider — not a hardcoded
Claude list that drifts out of date.

## Motivation

Claude’s offered models and marketing-style labels are maintained by hand in more than one
place. New models are missing until someone ships a list bump; display names go stale
while aliases may still work. Grok already surfaces what the agent advertises; Claude does
not. Oneshot work still pins cheap models in code, with no global user control.

Why now: multi-harness picking and oneshot chips already depend on an accurate model
catalog and clear labels. Keeping Claude on a static table means every catalog and oneshot
default change is a release-time edit instead of discovery.

## Intent

- Each available provider contributes models from its own discovery path; the host does
  not own a hand-curated Claude table as the source of truth

- At app start, known models for each available provider are refreshed into a
  process-local cache; discovery failure degrades gracefully (no panic; omit or keep a
  prior cache rather than hard-failing the app)

- Selectors always show human-readable names via a uniform display field; when a
  provider’s wire form is ugly, that provider transforms it — UI does not special-case
  harness name strings

- When a provider advertises a context window, the usage meter can use it; when it does
  not, fill stays absent rather than inventing a window

- Oneshot model is a **global, per-provider** setting (not per-project), with a sensible
  default chosen by string-matching the discovered list (e.g. prefer a cheap/fast id when
  present)

- When oneshot affordances are enabled in settings, the user can pick that oneshot model
  per provider from the cached catalog

- Title and reply oneshots for a harness use that provider’s configured oneshot model

## Non-goals

- Calling a public cloud model API as the primary catalog source
- Per-project oneshot model settings
- Changing the main-chat default cascade or the built-in default model
- Redesigning the chat model picker chrome beyond using the live catalog and display names
- Adding new harnesses
