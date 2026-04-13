# Project constitution schema

`project.md` captures **project-wide principles and constraints**
that agents should always consider. It acts as a constitution for
duckspec work: what is true regardless of which capability or change
you are looking at.

Optional. Projects that don't need one can omit it.

## Structure

```markdown
# <Project Name>

<1-2 sentence summary>

<freeform markdown content>
```

## Rules

- H1 title is required.
- A summary paragraph directly follows the H1.
- The body may contain any markdown.
- At most one `project.md` at `duckspec/project.md`.
- Edited directly — not carried through a change.

## Quality

- Write things that are **always true**: engineering principles,
  hard constraints, testing philosophy, out-of-scope boundaries.
- Don't duplicate what's in AGENTS.md or CLAUDE.md — those are
  agent configuration, this is project knowledge.
- Keep it concise. If project.md grows past two screens, some of
  its content probably belongs in codex entries.

## Example

```markdown
# acme-api

A REST API for the Acme product suite, written in Rust with a
strong preference for explicit error handling and small crates.

## Engineering principles

- **Filesystem is the source of truth.** No metadata in frontmatter
  or sidecars.
- **Library first.** Features start as library APIs and surface
  through CLIs only after the library is stable.

## Out of scope

- GUI applications (separate repo)
- Cloud-hosted variants
```
