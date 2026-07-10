# Change status coverage

How `ds status <change>` reports progress for the `test:code` scenarios a change
introduces: linked when a source `@spec` backlink resolves, open when it does not.

## Linkage

A scenario is linked or open by source backlinks alone:

```text
| Category | Condition                                      |
| -------- | ---------------------------------------------- |
| linked   | at least one resolving source @spec targets it |
| open     | no resolving source @spec targets it           |
```

The path list under a `test: code` marker (`> - path:line`) is not used for this
classification. A scenario whose marker lists paths but has no resolving source backlink
is open. A scenario with a correct source `@spec` is linked even if the marker path list
is empty.

Step task checkboxes do not affect linkage. Checked-vs-pending classification of unlinked
scenarios belongs to change-scoped audit, not status.

## What enters the snapshot

Only scenarios the change introduces, and only those that are `test:code`:

```text
| Source                         | Included                         |
| ------------------------------ | -------------------------------- |
| New change cap `spec.md`       | all of its test:code scenarios   |
| Spec delta                     | scenarios new after merge only   |
| Base cap scenario (unchanged)  | no                               |
| manual: / skip: / no test:code | no                               |
```

`test:code` follows the usual inheritance rule: a scenario’s own marker wins; otherwise
the requirement default applies.

## Change status presentation

`ds status <change>` surfaces the partition as progress: a linked count and a list of open
scenarios. Linked scenarios never appear under missing or open. Status remains a
dashboard — it does not fail the process over open work.

```text
ds status <change>
        │
        ▼
  change-introduced test:code
  × source @spec keys
        │
        ├─ linked  → progress fraction
        └─ open    → open list only
```
