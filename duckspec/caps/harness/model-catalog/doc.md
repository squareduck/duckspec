# Harness model catalog

Duckboard keeps a process-local catalog of models discovered from each available agent
provider. Chat and project model pickers, usage-meter windows, and oneshot model choices
read this catalog rather than rediscovering on every open.

## Lifecycle

```
app start
   │
   ▼
refresh each available provider
   │  success + non-empty → replace that harness’s slice
   │  empty / failure     → keep prior slice if any; else empty
   ▼
pickers / settings / usage meter  ←── read catalog only
```

Refresh runs once at application start (background is fine). This capability does not
require a second refresh when Settings opens.

## Per-harness slices

Models stay grouped by harness id inside the catalog. A failed or empty rediscovery for
one harness does not clear another harness’s slice. A harness that has never succeeded
stays empty until a successful discovery fills it.

## Selection source

Whatever the catalog holds is what the UI offers. Context-window lookup for a selected
model uses the matching catalog entry’s window when present; unknown windows stay unknown
rather than inventing a default.
