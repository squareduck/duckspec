# @ Harness selection

duckboard can drive more than one agent backend. Harness selection is the layer that keeps
track of *which* backend a model belongs to, routes each turn to that backend, and decides
which model a new turn prefers by default — and whether that choice is available to run.

## ~ Default resolution

The preferred model for a turn is resolved from a three-step cascade, most specific first:

```
per-chat pin  →  project override  →  global default
```

The first level that is set wins. A per-chat pin overrides a project override; a project
override overrides the global default. Clearing the project override means “use global,”
not “let the CLI pick.”

The preferred model is **available** only when it appears in the process model catalog.
When nothing is set at any cascade level, or the preferred model is absent from the
catalog, the turn has no available model: the chat model control shows **Missing**, and
send is blocked until the user picks a catalog model (or availability returns). The app
does not invent a substitute.

The global default is a concrete application setting. When it is still unset and the
catalog has models, it is seeded once: prefer the former built-in (`grok` / `grok-4.5`) if
present in the catalog, otherwise the first model in catalog order.

## @ Model identity

A model is identified by a pair: the harness that owns it and the model id within that
harness. This pairing is what gets persisted for a per-chat pin, a project override, and
the global default, so a stored choice is never ambiguous once models from several
backends share one list.

Older stored choices predate the harness dimension and record only a model id. These load
as the Claude harness, which was the only backend at the time they were written, so
existing pins keep working untouched.
