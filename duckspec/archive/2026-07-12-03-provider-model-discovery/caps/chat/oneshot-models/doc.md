# Chat oneshot models

Title summaries and optional reply-suggestion chips share a cheap oneshot path per
harness. Which model that path prefers is a global setting per harness — not a per-project
default and not the chat’s main model pin.

## Resolution

```
configured id in catalog  →  use it
else string-match default →  e.g. haiku / fast composer when present
else first catalog model  →  for that harness
```

Resolution always applies, including when agent input hints are off (titles still
oneshot). A preference that disappears from the catalog after an upgrade falls through the
same ladder.

## Settings

The Chat settings section keeps the agent input hints toggle. When hints are **on**,
Settings shows one oneshot model picker per harness that has models in the process
catalog; choices use display names and store the model id under the global per-harness
map. When hints are **off**, those pickers are hidden; stored values and string-match
defaults still apply to oneshots.

Project default model (main chat) stays separate and is unchanged by this capability.
