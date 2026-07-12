# Followup: Change list section counts

User-led QoL after implementation: show `(N)` counts on Change and Archived section
headers in the Changes area, like Ideas.

## Scope

Post-implementation followup on `archive-list-ux`. Ideas already formats headers as
`{label}  ({count})` (`area/ideas.rs`); Change list uses plain `"Change"` / `"Archived"`
collapsible titles (`area/change.rs`).

## Summary

```
| # | sev | lens | title | → next |
| --- | --- | --- | --- | --- |
| 1 | minor | quality | Section counts on Change and Archived | /ds-step |
```

## Issues

### 1. Section counts on Change and Archived - quality/minor

**Where:** Change list column — `"Change"` and `"Archived"` collapsible headers in
`crates/duckboard/src/area/change.rs`

**Why:** Ideas sections show `(N)` so density is scannable; Change/Archived hide how many
live vs archived rows sit under closed sections (especially Archived default-collapsed).

**Action:** Count rows the same way the lists are built (live non–idea-owned explorations
+ active changes for Change; `archived_entries` length for Archived). Format
`{label}  ({count})`. Leave Overview, Capabilities, Steps, Reviews, Files, etc.
unnumbered.

## Outcome

Single agreed QoL. Specs likely unnecessary (presentation-only). Plan via `/ds-step`,
implement via `/ds-apply`. Not archive-ready until this lands (or is explicitly deferred).
