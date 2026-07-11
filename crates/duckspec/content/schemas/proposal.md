# Proposal schema

A proposal is the **decision record** for a change: **what** we want and **why**
- distilled from exploration - before design and before capability layout. It is
not architecture, not a caps list, not an impact analysis, and not a sales
narrative. Those come later (design, then specs) or not at all.

## Structure

```markdown
# <Change Title>

<1-2 sentence summary>

## Motivation

<why this change, why now>

## Intent

<what should be true when this change succeeds - outcomes, behaviors, constraints
on the problem. User/system language, not module or capability paths.>

## Non-goals

<what this change deliberately does not try to solve>
```

Recommended sections, not enforced by `ds check` beyond H1 + summary.

## Rules

- H1 title is required
- A non-empty summary paragraph follows the H1 directly
- Body is freeform markdown; the Structure skeleton is the expected shape
- Path: `duckspec/changes/<change-name>/proposal.md`

## Quality

- **Motivation** states the problem and why now from agreed exploration - not a
  solution design and not a pitch for traction, adoption, or internal buy-in.
- **Intent** is the success picture: what becomes true, for whom, under what
  constraints. Stay above capabilities and code. Naming exact `caps/` paths or
  listing files is premature here; design and spec discover structure.
- **Non-goals** bound the problem so later stages do not silently expand it.
  Feature-level, not "we will not touch crate X" unless that *is* the product
  boundary.
- **Short and scannable.** Faithful summary of the decision, not a mini-design
  and not marketing copy.
- Body markdown follows `style` (load only if not already in context).

## Formatting

After write or edit: `ds format <path>`. Presentation follows `style` - load only
if not already in context.

## Example

```markdown
# Next-card composer hints

Align empty-composer next actions with trailing agent `next` meta cards so the
UI and templates share one authority for "what you can do next."

## Motivation

Templates already emit ranked `next` meta cards. Duckboard still offers a
separate disk-phase chip ladder and optional under-input suggestions, so
empty-composer authority is split and users see two systems.

Why now: meta-card syntax is stable; aligning the composer before more stages
depend on chips avoids dual brains becoming load-bearing.

## Intent

- After the first turn, empty-composer next actions come only from a trailing
  `next` meta card (ranked, capped)
- The active action appears as ghost text; empty Enter sends it; Tab cycles when
  there is more than one
- No trailing `next` after the first turn means no next-action ghost (missing
  offers are a template fix, not a UI fallback)
- Empty sessions may still seed one bootstrap action; that path ends after the
  first turn
- Optional oneshot suggestions stay under-input only - never empty Enter

## Non-goals

- End-to-end structured question-tool support
- Changing meta-card syntax or inventing new card kinds
- Redesigning the composer footer
- Auto-fixing missing `next` cards from disk phase after the first turn
```
