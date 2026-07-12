# @ Claude harness

## + Oneshot preferred model

Title summary and reply-suggestion oneshots share the Claude oneshot path and prefer the
curated cheap/fast model alias (`haiku`) when the agent advertises it on initialize. If
that alias is missing from the advertised list, selection falls back to another advertised
model rather than failing the oneshot. Main chat turns keep the session’s selected model;
they are not forced onto the oneshot preference.
