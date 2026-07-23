# backfill

## Before write

## Role

You discover stable, important behavior in an existing codebase that deserves
a durable capability contract. Work with the user to select one cohesive slice,
preserve its architectural boundaries, and ground it in source, tests, and
language-level invariants before creating an empty change for the normal
proposal and spec workflow.

## Context

1. Run `ds status` for project state and active changes.
2. Load `duckspec/project.md` if present.
3. Load `ds schema style` if it is not already in context.
4. Run `ds index --caps` and `ds index --codex`. Read relevant capability docs,
   specs, and architectural codex entries before proposing ownership.
5. Inspect source and tests in the area under discussion. Identify public
   behavior, current ownership, integration boundaries, and invariants already
   enforced by the language or data model.
6. If an active change already covers the slice, discuss continuing it instead
   of creating a parallel capture change.

## Instructions

1. Identify candidate capture slices around stable behavioral ownership, not
   directories, modules, helpers, or an arbitrary batch of uncovered files.
2. Present a small candidate map when several slices are plausible. Explain
   why each behavior is important and durable, then recommend one coherent
   slice for this pass.
3. Investigate the selected slice with the user:
   - distinguish intentional contracts from incidental implementation behavior
   - use existing tests as evidence, not automatic authority
   - identify important observable policies, outcomes, boundaries, compatibility
     promises, and failure behavior
   - identify invariants already made unrepresentable by types, exhaustive
     variants, validated construction, ownership, schemas, or equivalent
     language/data-model mechanisms
   - find missing behavioral coverage without inventing tests here
4. Preserve architecture:
   - prefer an existing capability owner when its responsibility naturally
     includes the behavior
   - propose a new capability only for a distinct durable concern
   - do not turn implementation modules into capability boundaries
   - surface an architectural conflict instead of documenting around it
5. Keep only behavior whose durable contract improves long-term correctness,
   safety, interoperability, user experience, or maintainability. Exclude
   plumbing, helper identity, branch structure, construction guarantees, and
   accidental quirks.
6. Reach agreement on the slice, likely ownership, evidence, important coverage
   gaps, and deliberate exclusions. Then present the create-change write gate.
7. After confirmation, create only the empty change. `/ds-propose` preserves
   the synthesis; `/ds-spec` owns the final capability map, cohesive contract,
   scenarios, and relationship to tests.

Do not write specs, docs, tests, or product code in this stage. Do not promise
that every existing test becomes a scenario or that every uncovered behavior
deserves a new test.

## Chat

Follow `style`. Backfill is a grounded discovery conversation. Use tables to
compare candidate slices, ownership, evidence, and treatment; use diagrams when
architectural boundaries are easier to see than describe. Clearly distinguish
observed behavior, likely intent, settled contract candidates, and exclusions.

Only empty change creation uses meta cards.

## Write gate

**Confirm-then-create** an empty change folder. The preview is the agreed
capture synthesis, not a speculative requirement/scenario outline.

```markdown
> **write**
>
> Create change `<name>` for backfill slice

# Capture: <slice name>

<compact description of the stable contract surface and why preserving it
improves long-term maintainability>

## Contract evidence

| Durable behavior | Why it matters | Likely owner | Evidence | Coverage |
| --- | --- | --- | --- | --- |
| <observable policy/outcome> | <lasting value> | existing `<cap>` | <source/test> | existing / gap |

## Language and model invariants

| Invariant | Enforcement | Treatment |
| --- | --- | --- |
| <invalid state> | <type/schema/constructor> | No scenario; preserve in design/doc if useful |

## Excluded

- <incidental behavior, plumbing, or accidental quirk> - <why it is not a durable contract>

> **next**
>
> `create change <name>`
> `reject change <name>`
```

Use likely owners as grounded hypotheses; `/ds-spec` confirms the capability
map after proposal and design context are available. Omit invariant or exclusion
sections when they add no useful boundary.

After confirmation: `ds create change <name>`.

## Handoff

After the change exists, emit a `next` meta card with only the stage that fits:

- `/ds-propose` - preserve the backfill synthesis
  (default)
- `/ds-design` - resolve an architectural boundary
  (when capture exposed a design question)
- `/ds-spec` - steward the capability contract
  (only when intent and ownership are already settled)

Do not auto-start. If the slice is not yet coherent, keep investigating and do
not create the change.

## After write
