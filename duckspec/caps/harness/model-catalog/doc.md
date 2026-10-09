# Harness model catalog

Duckboard keeps a process-local catalog of models discovered from each available agent
provider. Chat and project model pickers, usage-meter windows, and oneshot model choices
read this catalog rather than rediscovering on every open.

Every harness refreshes at app start. Grok and Codex clear when rediscovery is empty or
fails, and are not refreshed again. Claude keeps the last good slice across a failed
refresh, and is refreshed again at catalog expiry and while that slice is stale or empty.

## Lifecycle

Refresh runs at application start for every available provider. Opening Settings does not
refresh the catalog.

```
app start
   │
   ▼
refresh each harness once
   │
   ├── grok, openai-codex
   │     non-empty success → replace that slice
   │     empty or failure  → clear that slice
   │     no later refresh
   │
   └── claude
         models returned      → replace slice, memo, and snapshot
         empty success        → clear slice and memo; leave the snapshot file
         fetch failed         → memo, else applied slice, else snapshot, else empty
         │
         while the app is open
         expiry time          → one wake, Claude only
         empty or past expiry → retry every five minutes, Claude only
         fresh, or no expiry  → no poll; no expiry lasts until the next launch
```

The snapshot is `~/.config/duckboard/claude-catalog.json`. A missing or unreadable file is
no snapshot. Rows load as stored, including effort when the stored row has it, as the
`claude-code` harness. Load does not filter them again. The next successful fetch applies
the current binary’s filter.

## Per-harness slices

Models stay grouped by harness id inside the catalog. A refresh for one harness leaves the
other harnesses’ slices alone. Grok and Codex drop their slice when rediscovery is empty
or fails. Claude follows the retention ladder above: a failed refresh can keep a prior
slice, and an applied empty slice stays empty until a later success fills it.

## Selection source

Whatever the catalog holds is what the UI offers. Context-window lookup for a selected
model uses the matching catalog entry’s window when present; unknown windows stay unknown
rather than inventing a default.
