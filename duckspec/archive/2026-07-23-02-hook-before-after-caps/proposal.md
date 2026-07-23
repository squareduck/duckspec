# Hook before/after + caps

Align hook scaffolding with the live `before`/`after` vocabulary and backfill a dedicated
`cli/hooks` capability so create, filenames, and template injection share one product
surface.

## Problem

Per-stage hooks already render with a shared vocabulary: files under
`duckspec/hooks/<stage>-before.md` / `-after.md` inject into `## Before write` /
`## After write` when `ds template` runs. README and stock templates use that naming. The
create path never moved:

```
ds create hook <stage> --pre|--post
        │
        ▼
hooks/<stage>-pre.md | -post.md   ← scaffolded, never loaded

ds template <stage>
        │
        ▼
looks for hooks/<stage>-before.md | -after.md
```

A freshly created hook is invisible to template rendering until someone renames the file.
Plan-layer `HookPosition::{Pre,Post}` and skeleton H1s (`# Explore - Pre`) teach the same
stale names. There is no capability that owns hook create, stage set, empty/missing rules,
or injection behavior — only a side mention under `cli/stock-content`.

## Direction

- Hard-break CLI and plan to `before` / `after`: flags `--before` / `--after`, filenames
  `*-before.md` / `*-after.md`, skeleton labels and tests updated. No aliases for `--pre`
  / `--post`.

- Add `cli/hooks` (`spec.md` + `doc.md`) covering create rules, known stages, template
  injection, empty/whitespace-only as absent, and the optional `hooks/` directory.

- Leave `cli/stock-content` as a cross-reference only, not the owner of hook rules.

## Boundaries

```
| In | Out |
| --- | --- |
| Rename create/plan/tests to `before`/`after` | Soft compatibility or aliases for `pre`/`post` |
| Capability backfill for existing hook behavior + the rename | Multiple hooks per stage or cross-stage composition |
| Single vocabulary end-to-end (flags → files → section headers) | Audit of hook files; duckboard `ContextHook` (different concept) |
| | Rewriting `references/duckspec.md` research notes unless later needed |
```

This repo’s only project hook (`hooks/apply-after.md`) already uses the new naming, so
in-tree migration is not a blocker. External users of `--pre`/`--post` or `*-pre.md` /
`*-post.md` must update — accepted as a hard break.
