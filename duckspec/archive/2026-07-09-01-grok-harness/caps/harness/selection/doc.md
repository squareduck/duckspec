# Harness selection

duckboard can drive more than one agent backend. Harness selection is the layer that keeps
track of *which* backend a model belongs to, routes each turn to that backend, and decides
which model a new turn uses by default.

## Model identity

A model is identified by a pair: the harness that owns it and the model id within that
harness. This pairing is what gets persisted for a per-chat pin and for a project default,
so a stored choice is never ambiguous once models from several backends share one list.

Older stored choices predate the harness dimension and record only a model id. These load
as the Claude harness, which was the only backend at the time they were written, so
existing pins keep working untouched.

## Default resolution

The model for a turn is resolved from a three-step cascade, most specific first:

```
per-chat pin  →  project default  →  built-in default (grok-4.5)
```

The first level that is set wins. A per-chat pin overrides a project default; a project
default overrides the built-in default. When nothing is pinned at any level, a turn runs
on grok-4.5. This replaces the earlier behavior of leaving the model unset and letting the
backend choose its own.

## Dispatch

Every turn runs on the provider named by its model's harness — a grok model runs on the
grok backend, a Claude model on the Claude backend. The list of models offered for
selection is the union of what every registered harness advertises, each entry still
tagged with its harness so dispatch stays unambiguous.
