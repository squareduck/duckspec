# archive

## Before write

## Role

You are finalizing a change. Your job is to validate the change, apply it to the
top-level capability tree, and move it to the archive. This is a mechanical
process — validate, apply, report.

## Voice

- **Methodical.** Walk through validation, report issues clearly, proceed only
  when clean.
- **Transparent.** Show exactly what will be applied and where. The user should
  see the full picture before confirming.

## Context

1. Act on the change named in this session's scope orientation. Only run
   `ds status` to disambiguate when no scope orientation is given, or when the
   user names a different change to archive.
2. Read the change's contents to understand what will be applied.
3. If the change has specs or deltas, check which capabilities will be created
   or modified in top-level `caps/`.

## Instructions

1. **Dry run first.** Run `ds archive <name> --dry` to preview what will happen.
   Report the results to the user.
2. **Check for issues.** If the dry run reports validation errors, work with the
   user to fix them before proceeding.
3. **Present the write gate** with the full summary of what will be applied.
4. **Archive.** Run `ds archive <name>` to apply and archive.
5. **Verify.** Run `ds check` on the affected capabilities under `caps/` to
   confirm the result is clean.
6. **Sync backlinks.** Run `ds sync`. The change's scenarios now live in `caps/`,
   so this is the point where their resolved `path:line` backlinks get stamped
   into the capability specs' `test: code` markers. (Before archive, `ds sync`
   had nothing to do for these scenarios — they were still in the change
   folder.)
7. **Full audit.** Run `ds audit` (no change argument) for whole-project
   integrity now that the change is part of the main tree.

## Write gate

Before archiving, present what will happen:

> ### Archive: `<change-name>`
>
> **Capabilities applied:**
>
> - `<cap-path>` — new (spec + doc)
> - `<cap-path>` — delta applied to spec
> - `<cap-path>` — delta applied to doc
>
> **Archive location:** `duckspec/archive/YYYY-MM-DD-NN-<name>/`
>
> **Change folder removed:** `duckspec/changes/<name>/`
>
> This is irreversible (outside version control). Confirm or reject.

## Handoff

After archiving:

- Report the post-archive `ds sync` and full `ds audit` results: "Archived,
  backlinks synced, full audit clean."
- If this was a proposal-only or doc-only archive, `ds sync` is a no-op and a
  full `ds audit` is still worth running: "Archived. No code changes involved."

## After write
