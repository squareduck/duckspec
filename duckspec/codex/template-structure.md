# Template structure

Canonical section shape for every duckspec agent template. Principles for what templates
own and how they behave live in `template-and-schema-authoring`; this entry is only the
skeleton and what each section holds.

## Skeleton

Sections appear in this order. Every template has every section.

```markdown
# <stage>

## Before write

## Role

## Context

## Instructions

## Chat

## Write gate

## Handoff

## After write
```

`<stage>` is the template name (e.g. `explore`, `propose`, `spec`) - matching the
`ds template <stage>` identifier.

## Before write

Hook injection point. Leave empty in the source template. At render time, `ds template`
may inject project hook content here.

## Role

Two to four sentences: who the agent is for this stage and what sole job it has. Names the
outcome of a successful run (artifact, decision, report) when there is one. Include only
behavioral constraints essential to the job. Put concrete collaboration behavior under
Chat and reasoning priorities under Instructions; do not add a personality or voice
section.

## Context

Numbered load sequence only: what to read or run, in order, before acting. Follows the
progressive-loading pattern in `template-and-schema-authoring` (scope/status → project.md
→ stage inputs → `ds schema style` if needed and not already loaded → just-in-time
artifact schemas → adjacent lookup). Each step is one action; schema loads are
`ds schema <name>` pointers, not pasted bodies. Style is loaded at most once per session.

## Instructions

The stage spine: a short numbered list of what to do once context is loaded. High altitude
- main path of the stage. Judgment that is stage-specific may appear as a brief bullet;
mechanical artifact rules stay in schemas; markdown presentation stays in `style`.

## Chat

How this stage uses chat and markdown presentation. Points at `ds schema style` (load only
if not already loaded). States when this stage emits ordinary information display and when
it emits a `write` meta card or a `next` meta card. Always use those full names (see
`style`). Stage-specific notes only - no second copy of the style guide.

## Write gate

What must happen before or around writes in this stage. Describes the gate kind for this
stage (confirm-then-write, document-only, no write, execute) and what the agent shows the
user. References meta card composition from `style` (`write` meta card + preview + `next`
meta card with `confirm` / `reject`) when this stage waits on approval. Concrete enough
that the agent knows what to present; not a full artifact schema.

## Handoff

When the stage is done: how to close and whether to emit a `next` meta card. Points at
`style` for encoding. Describes typical send tokens for this stage (slash commands, bare
tokens) as guidance for the agent’s choice - not a fixed decision tree. Notes when
omitting the `next` meta card is correct.

## After write

Hook injection point. Leave empty in the source template. At render time, `ds template`
may inject project hook content here.
