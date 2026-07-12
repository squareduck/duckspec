# @ Claude harness

## ~ Oneshot preferred model

Title summary and reply-suggestion oneshots share the Claude oneshot path. They use the
preferred oneshot model resolved for the Claude harness (global setting or string-match
default) when the agent advertises that model on initialize. If the preferred model is
missing from the advertised list, selection falls back to another advertised model rather
than failing the oneshot. Main chat turns keep the session’s selected model; they are not
forced onto the oneshot preference.

## + Model discovery

The host does not own a static Claude model table. Claude models offered for selection
come from what the owned agent advertises on ACP initialize: each entry is tagged as the
Claude harness, carries a human-readable display name, and may carry a context window when
the agent knows one.

```
live discovery succeeds  →  advertise live catalog on initialize
live discovery fails     →  advertise curated alias fallback
host list_models         →  that advertise set (or empty if discovery cannot run)
```

Live discovery uses credentials available to the official `claude` install. The host only
reads the initialize result; it does not call the model catalog API itself. When the host
cannot obtain an advertise set at all, the listed set is empty and the app does not panic.
