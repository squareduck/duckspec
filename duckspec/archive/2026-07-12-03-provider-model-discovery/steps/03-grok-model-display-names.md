# Grok model display names

Ensure every model returned by the Grok harness carries a non-empty human-readable display
name (prefer advertised name; light fallback for bare ids).

## Tasks

- [x] 1. Apply provider-local display humanization in Grok’s `to_model_info` (or
         equivalent mapping from handshake models)

- [x] 2. Align oneshot preferred-model wording with the shared preferred-if-advertised
         path if any constants/docs still hardcode “cheapest only”

- [x] 3. @spec harness/grok Model discovery: Each listed model carries a display name
