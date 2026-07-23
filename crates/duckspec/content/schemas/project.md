# Project schema

`duckspec/project.md` is the short high-level orientation to what the project
is: its purpose, identity, and major shape. It should be quick for a user or
agent to absorb at the start of a session.

## Structure

```markdown
# <Project Name>

<1-2 sentence summary>

<body>
```

Body is freeform markdown (headings, lists, tables, diagrams, prose).

## Rules

- H1 title is required
- A non-empty summary paragraph follows the H1 directly
- No structural rules on the body beyond parseable markdown
- At most one file: `duckspec/project.md`
- Edited in place - no deltas, no change folder copy

## Quality

- **Immediate orientation.** Explain what the project does, its domain, and the
  major components or surfaces a newcomer needs to recognize.
- **High-level shape only.** Mention technologies when they materially orient
  the reader. Detailed architecture, testing strategy, UI guidance,
  engineering conventions, glossaries, and operating procedures belong in
  codex.
- **No capability detail.** Capability behavior and reader documentation belong
  in paired specs and docs.
- **Not agent config.** Do not duplicate `AGENTS.md` / harness instructions;
  this describes the project, not how the agent should operate.
- **Stable and short.** Exclude current work, history, backlog, and session
  narrative. Keep it compact enough to absorb during routine orientation.
- Body markdown follows `style` (load only if not already in context).

## Formatting

After write or edit: `ds format <path>`. Presentation follows `style` - load only
if not already in context.

## Example

```markdown
# acme-api

REST API and worker system for the Acme billing suite.

## Shape

- `acme-api` accepts account, invoice, and payment requests
- `acme-worker` processes asynchronous billing and notification jobs
- PostgreSQL is the shared durable store
- The web dashboard and public API use the same application services
```
