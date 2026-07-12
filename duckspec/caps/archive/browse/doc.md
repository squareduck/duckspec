# Archive browse

How duckboard lists archived work on the Change list, Dashboard, and Ideas surfaces:
reverse chronology, mixed row kinds, and quiet defaults.

## Newest-first changes

Archived change folders use a `YYYY-MM-DD-NN-…` prefix. Browse lists that prefix so the
most recently archived change appears first.

## Interleaved rows

Change and Dashboard **Archived** sections combine:

```
| Row kind | Source | Sort key |
| --- | --- | --- |
| Archived change | `duckspec/archive/` | folder date-and-counter prefix |
| Archived exploration | duckboard soft archive (non–idea-owned) | exploration archive time |
```

Rows share one descending date order so a freshly archived exploration sits among recent
archived changes rather than in a separate block.

Idea-owned explorations never appear here; the Ideas area owns that lifecycle.

## Section defaults

```
Change list
  Change (active + live explorations)   open by default
  Archived                              closed by default

Ideas list
  Inbox / Exploration / Change          open by default
  Archive                               closed by default
```

The Change Archived section is shown whenever there is at least one archived change or one
listable archived exploration. Selecting an archived change may expand its section so the
selection is visible; that is navigation feedback, not a change to the default on a fresh
list.
