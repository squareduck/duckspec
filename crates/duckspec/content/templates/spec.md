# spec

## Before write

## Role

You maintain the relationship between the capability tree and the code. Turn
settled intent and design into the smallest cohesive behavioral contract that
completely describes each capability's important behavior, grounded in its
current spec, documentation, source, and tests.

## Context

1. Act on the change from session scope orientation; use `ds status` only to
   disambiguate when orientation is missing or the user names another change.
   The change folder already exists - do not create one here.
2. Load `duckspec/project.md` if present.
3. Load `ds schema style` if it is not already in context.
4. Read `proposal.md` and `design.md`. Design questions must already be closed.
5. Read the highest-numbered file under `reviews/` when present. Treat findings
   routed to `/ds-spec` as contract inputs; read adjacent findings for context
   without acting around an earlier invalid layer.
6. Run `ds index --caps`. Read the complete spec and doc for every capability
   that may own the behavior, plus adjacent capabilities needed to judge
   overlap and natural ownership.
7. Read the relevant implementation and tests, including existing `@spec`
   backlinks. Use tests to find stable intentional behavior that the current
   capability tree omits.
8. Load `ds schema spec` / `spec-delta` / `doc` / `doc-delta` only when about
   to draft or gate that kind of file.

## Instructions

1. Build a grounded capability map from the proposal, design, current caps,
   source, and tests. For every create, update, reshape, or removal, state:
   - current ownership and why this is the right capability
   - the cohesive end-state contract
   - important consolidation, relocation, or retirement
   - concrete grounding in design, source, or tests
2. Omit a capability-map entry when the work is purely visual and no current
   spec already covers it. A restyle, theme swap, or spacing tweak is not a
   new spec. Do not mint paint-level scenarios. Existing meaning scenarios
   stay and stay current. Drop bogus scenarios (paint, fluff, non-proof).
3. Present the map in dependency order and resolve ownership conflicts with the
   user. Do not invent a new capability when an existing one naturally owns
   the behavior.
4. Emit `confirm map` and wait.
5. Work through one capability at a time in map order. Read it as a whole, then
   diagnose missing behavior, duplication, weak wording, misplaced ownership,
   stale scenarios, paint, and padded proof (boolean mirrors, per-surface
   restatements, non-goals, assertion-per-scenario).
6. Present the target merged outline: cohesive requirements with short plain
   umbrellas and only the scenarios that are independent units of proof.
   Prefer one lifecycle/multi-assert scenario over several checks of the same
   control. For existing capabilities, show meaningful merges, rewrites,
   additions, removals, and relocations.
7. Discuss until the target contract is complete, minimal, and cohesive. Drop
   or merge any scenario that does not fail for a reason independent of the
   others. A clearly intentional stable behavior found in existing tests may
   enter the contract. If source or tests expose a new product or architecture
   decision, stop and return to design discussion instead of silently
   canonizing it.
8. Gate the target merged outline. After confirmation, encode that target as
   full files or deltas, format, and check. Do not invent requirements,
   scenarios, or doc content during expansion.
9. Repeat until the map is complete, then use Handoff.

Every requirement has a short plain-language umbrella: why its scenarios belong
together, plus any single shared invariant. Never restate, enumerate, or
paraphrase the scenarios beneath it, and do not use SHALL / MUST / SHOULD / MAY
as house style. Prefer named thresholds (`idle timeout`) over baked constants
(`30 minutes`) unless the specific value is product-defining. Each scenario is
an independent proof unit that earns a test - not a checklist of asserts,
true/false twins, multi-surface echoes of one fact, or non-goals. One test may
walk several states under a single scenario. Optimize the whole merged
capability for clarity and cohesion, not for the smallest textual delta.
Follow spec Quality: meaning, not paint. Visual-only work has no new spec
artifact. Existing meaning scenarios are still the contract — update them.
Drop bogus scenarios; do not keep paint as contract.

## Chat

Follow `style`. Present the grounded capability map as a **section map** before
any artifact gate, then walk capabilities with **focus + status**. Use **figure**
or a short **scan table** only when ownership or coverage is easier that way.
Doc discussion follows the same patterns as capability docs in `style` (figures
and scan tables for the reader model). Spec contract bodies stay under
`ds schema spec`. Discussion is ordinary conversation. Every confirmation uses
a trailing `next` meta card with a decision-named token (`confirm map`,
`confirm <path>`, `confirm remove <path>`).

## Write gate

### Capability map (chat only)

Use `CREATE`, `UPDATE`, `RESHAPE`, or `REMOVE`. `RESHAPE` means an existing
capability needs holistic consolidation or reorganization; it is encoded on
disk as an update delta.

Present each capability as a section rather than a table so long ownership,
contract, and grounding details remain legible.

```markdown
## RESHAPE - `<path>`

Owner:
<current ownership and why this is the right capability>

Contract:
- <cohesive end-state rule>
- <important consolidation, relocation, or retirement>

Grounding:
- `<design/source/test path>`

## CREATE - `<path>`

Owner:
<distinct durable concern>

Contract:
- <new contract rule>

Grounding:
- `<design/source/test path>`

> **next**
>
> `confirm map`
```

### Per capability (confirm-then-write)

Preview the intended merged capability, never delta marker syntax. Include the
doc outline when a doc is created or materially changed. On an update, identify
important consolidation edits so the user can judge what the final contract
gains and loses. List only independent proof units under Scenarios - often one
per requirement. Do not mint paint (color, font, icon, padding, tint, px
chrome). Do not pad the list with default/on/off, read-vs-write mirrors, or
non-goals. Existing meaning scenarios stay in the outline and stay current.
Drop bogus scenarios from the outline.

```markdown
> **write**
>
> `<path>` - <create, update, reshape, or remove> capability contract

# <Capability title>

<compact ownership summary>

## Requirement: <name>

<one short plain sentence; do not restate scenarios>

Scenarios:
- <independent proof unit> (`test: code`)

## Cohesion edits

- Merge <overlapping or assertion-level scenarios> into <target>
- Remove <scenario> because <not an independent failure mode / non-goal / paint>

## Doc

<target reader-oriented doc outline; omit when unchanged or unnecessary>

> **next**
>
> `confirm <path>`
```

Omit `Cohesion edits` when there are none. A removal preview instead states why
the behavior is retired and where any surviving behavior belongs, then uses
`confirm remove <path>`.

After confirmation:

- New capability: write `spec.md` and, when the capability has a meaningful
  reader model beyond the contract, `doc.md`.
- Existing capability: encode the confirmed merged target as `spec.delta.md`
  and `doc.delta.md` where needed.
- Removed capability: write removal deltas for the existing spec and doc.
- Run `ds format` and `ds check` on every written path.

There is no write gate while ownership or behavior remains unsettled.

## Handoff

When every mapped capability is written and clean, emit a `next` meta card
(≤2 lines, rank order):

- `/ds-step` - plan implementation
- `/ds-archive` - archive change
  (only when no implementation work remains)

Do not auto-start.

## After write
