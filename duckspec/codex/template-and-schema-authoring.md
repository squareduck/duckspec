# Template and schema authoring

Principles for authoring duckspec agent templates and artifact schemas so every stage
stays consistent, useful, and small - without overspecifying behavior.

## Ownership

Templates and schemas split a clean line: **process vs shape**. Shared markdown style is
neither - it lives once in `style`.

```
| Concern                         | Lives in   |
|---------------------------------|------------|
| Role, voice, process            | template   |
| Progressive load order          | template   |
| When to emit `write` / `next` meta cards | template |
| Write gate and handoff *when*   | template   |
| Artifact structure and rules    | schema     |
| Artifact quality bar            | schema     |
| Artifact example (if any)       | schema     |
| How markdown should look        | style      |
```

- **Template** - who the agent is for this stage, what to load, what to do, when to gate
  or hand off. It does not restate artifact grammar or the style guide.

- **Schema** - what a valid on-disk artifact looks like and what “good” means for it. It
  does not restate CLI workflow, voice, or handoff. Artifact schemas follow
  `schema-structure`. They point at `style` for prose, tables, and diagrams rather than
  duplicating presentation rules.

- **`style` schema** - **not** an artifact schema. One small guide for markdown
  **everywhere**: chat messages and artifact bodies. Ordinary information patterns plus
  rare chat-only meta cards. Own structure (not Structure/Rules/Quality/Formatting/
  Example). Load with `ds schema style` **only if it is not already loaded** this session.
  Templates and schemas both reference it; whoever needs it first loads it.

If a sentence tells the agent how to behave in conversation or which command comes next,
it belongs in a template (or points at `style` for presentation). If it tells the agent
what a valid file looks like, it belongs in an artifact schema.

Companion entries: `template-structure` (templates), `schema-structure` (on-disk artifact
schemas only). This entry is principles only.

## Altitude

Write at the **right altitude**: specific enough to steer, loose enough to let the model
think.

- Prefer short role text, a handful of voice qualities, and a short instruction spine -
  not branch-by-branch scripts.

- State judgment heuristics (“prefer deltas”, “economical scenarios”) as brief principles.

- Let `ds check`, parsers, and tools own mechanical correctness. Do not re-document every
  parse error in a template.

- Minimal nudge on handoff: offer choices, do not auto-start the next stage or funnel the
  user through a fixed path.

Size is not a hard budget. Thinness follows from altitude and ownership - if content is
duplicated across template, schema, and `style`, delete the copies.

## Progressive loading

Every template’s Context section loads information **just in time**, in a stable order:

1. Scope orientation / `ds status` when needed to know which change and stage

2. `duckspec/project.md` when present

3. Stage-specific inputs (proposal, design, step, specs, …)

4. `ds schema style` if markdown will be written and style is not already in context

5. `ds schema <name>` only when about to draft or validate that artifact (artifact schemas
   assume style may already be loaded; they still say “follow `style`”)

6. Adjacent lookup (`ds index`, caps, source) as needed - not a full dump

Do not paste schema or style bodies into templates. Point and load. **Load `style` at most
once per session** - later templates and schemas only require following it, not reloading
it.

## Write gates

Every template has a Write gate section. The gate always exists; what it does depends on
the stage:

- **Confirm-then-write** - show the intended artifact shape, wait for approval, then write
  (propose, design, spec, …)

- **Document-only** - the only write is the stage’s record (review, followup); no silent
  edits to plan or product code

- **No write** - diagnose or explore without creating files (verify; explore until a
  change is requested)

- **Execute** - apply works through tasks; gate only on ambiguity or surprise

Gates present intent in chat using `style` meta cards when confirming a write. They do not
restate the full artifact schema.

## Markdown and chat

All markdown - in chat and in artifacts - should be **clean and consistent**, not
necessarily brief. Depth is fine when it clarifies; noise is not.

Two layers (full detail in `ds schema style`):

1. **Information** (almost everything) - normal GFM shaped by the data: tables, diagrams,
   lists, prose. Same taste in a file as in a message.

2. **Meta cards** (chat only, rare) - the `write` meta card and the `next` meta card only.
   Not used in artifacts. Not used for findings, triage, or reports.

If it can be a good table or ordinary prose, it is not a meta card. Always name them
**`write` meta card** and **`next` meta card** in templates so the agent is never confused
with prose “next steps.”

Templates state *when* this stage uses meta cards or particular information shapes.
Schemas state *what* the file must contain and point at `style` for how body markdown
should read. Neither restates the style guide.

Clients may parse a trailing `next` meta card as quick actions. The agent chooses which
actions fit the conversation - not a fixed disk-phase tree.

## Examples

Artifact schemas may include **zero or one** example.

- Include an example when the shape is easy to misread from rules alone, or when a
  multi-part combination is the common failure mode.

- Prefer one canonical example over an edge-case catalog. Mechanical errors belong to
  `ds check` and parser tests.

- **Deltas:** rules plus one small multi-marker example (e.g. anchor, add child,
  rename+modify) beat a second full happy-path novel. Judgment (`@` vs `~`, lightest
  touch, cold-reader doc bodies) stays in Quality bullets.

- Examples follow `style` (tables, fences, prose) like any other markdown.

Templates do not carry long artifact examples; they load the schema.
