# Scope-aware templates

Reword the agent-command templates so they default to the session's scoped change and
treat `ds status` as a disambiguation fallback.

## Context

The templates live under `crates/duckspec/content/templates/`. `archive.md` and `apply.md`
open their Context section with "Run `ds status` to identify the change", which is
ambiguous when several changes are active. This is prose content, not a `caps/`
capability, so there is no `@spec` task here.

## Tasks

- [x] 1. Reword `templates/archive.md` so it acts on the change named in the session's
         scope orientation, using `ds status` only to disambiguate when no scope is given
         or the user names a different change

- [x] 2. Apply the same rewording to `templates/apply.md`

- [x] 3. Grep the remaining templates for "identify the change" / "Run `ds
                status`"
         and apply the same default-to-scoped-change wording to any sibling that
         re-derives the change from `ds status`
