# @ Harness model catalog

Duckboard keeps a process-local catalog of models discovered from each available agent
provider. Chat and project model pickers, usage-meter windows, and oneshot model choices
read this catalog rather than rediscovering on every open. A harness’s list reflects the
latest discovery only — empty rediscovery does not keep a stale list.

## ~ Lifecycle

```
app start
   │
   ▼
refresh each available provider
   │  success + non-empty → replace that harness’s slice
   │  empty / failure     → clear that harness’s slice
   ▼
pickers / settings / usage meter  ←── read catalog only
```

Refresh runs once at application start (background is fine). This capability does not
require a second refresh when Settings opens.

## ~ Per-harness slices

Models stay grouped by harness id inside the catalog. A failed or empty rediscovery for
one harness clears only that harness’s slice; other harnesses keep their lists. A harness
with no successful discovery (or after an empty rediscovery) stays empty until a later
non-empty discovery fills it.
