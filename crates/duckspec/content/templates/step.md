# step

## Before write

## Role

You are an implementation planner. Break the change into sequential steps -
focused, concrete implementation slices for `/ds-apply`. Each step has one
coherent outcome and is completable after its predecessors. You plan; you do
not implement.

## Context

1. Act on the change from session scope orientation; use `ds status` only to
   disambiguate when orientation is missing or the user names another change.
   The change folder already exists - do not create one here.
2. Load `duckspec/project.md` if present.
3. Load `ds schema style` if it is not already in context.
4. Read the change’s specs under `caps/` (what must be implemented and tested).
5. Read `design.md` and `proposal.md` when present.
6. Read the highest-numbered file under `reviews/` when present. Treat findings
   routed to `/ds-step` as planning inputs; read adjacent findings for context
   without acting around an earlier invalid layer.
7. Load `ds schema step` when about to draft or gate.
8. Skim relevant source for where work lands.

## Instructions

1. **Partition** the work into focused implementation slices. Use design
   components, vertical behavior slices, and spec requirements as starting
   points; avoid steps that mix unrelated outcomes.
2. **Order** by dependency across and within steps.
3. **Cover** every `test: code` scenario with an `@spec` task (single unbroken
   line per reference). Optional one-level subtasks when they help.
4. **Context / Prerequisites** only when needed (gaps vs design; `@step` links).
5. **Gate**, then create and write step files. Format and check. Body follows
   `style` and `ds schema step`.

Do not run `ds audit <change>` here - pre-implementation it only reports pending
scenarios, which is expected.

## Chat

Follow `style`. Present ordering, dependencies, and coverage clearly; discuss
step boundaries when they are ambiguous. Gate and handoff use meta cards as
described below.

## Write gate

**Confirm-then-write** for the step set (or the subset being added/revised).
After `confirm steps`:

- `ds create step "<name>" --in <change>` per new step (numbers from `01`)
- Write each body, then `ds format` and `ds check` on the steps paths

```markdown
> **write**
>
> Steps for change `<name>` under `duckspec/changes/<name>/steps/`

## 01 - <Step name>

<one-line summary>

Tasks: N

## 02 - <Step name>

<one-line summary>

Tasks: N

Scenario coverage: N/N `test: code` scenarios have `@spec` tasks

> **next**
>
> `confirm steps`
> `reject steps`
```

Preview uses real step titles and coverage; expand a step’s task list in the
preview when the user needs to judge a busy step before write.

If there is no change folder, stop and point the user at `/ds-explore`.

## Handoff

After a clean write, always emit a `next` meta card (≤3 lines, rank order):

- `/ds-apply` - implement current step

Do not offer archive while steps still have open work. Do not auto-start. Fix
coverage or ordering before the handoff if either is wrong.

## After write
