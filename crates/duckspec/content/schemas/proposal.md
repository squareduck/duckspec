# Proposal schema

A proposal is the durable synthesis of an exploration: the clearest compact
record of what was learned and settled before the chat disappears. Its body is
freeform so the subject, not a ceremonial outline, determines the shape.

## Structure

```markdown
# <Change Title>

<compact summary>

<freeform body shaped to the subject>

## Open questions

<optional; only when genuine uncertainty is part of the decision context>
```

The body may use prose, headings, tables, lists, or ASCII diagrams in any
combination that communicates the exploration clearly. `Open questions` is
optional, not part of the default shape.

## Rules

- H1 title is required
- A non-empty summary paragraph follows the H1 directly
- Body after the summary is freeform markdown
- If present, `## Open questions` contains only uncertainty that remains
  material to the direction of the change
- Path: `duckspec/changes/<change-name>/proposal.md`

## Quality

- Preserve the conclusions and context a future reader cannot recover once the
  exploration chat disappears.
- Explain the problem and intended improvement without replaying the
  conversation or repeating the same point under several headings.
- Include boundaries, constraints, rejected directions, or open questions only
  when they were material to the exploration.
- Prefer concrete project language and visual structure over padded rationale,
  generic benefits, or a sales narrative.
- Stay above implementation planning, capability layout, and file impact;
  design and spec discover those later.
- Keep the record compact, clear, and informative rather than merely brief.
- Body markdown follows `style` (load only if not already in context).

## Formatting

After write or edit: `ds format <path>`. Presentation follows `style` - load only
if not already in context.

## Example

````markdown
# Calm chat transcript

Make agent turns easy to scan by separating thinking, tool activity, and the
answer instead of flattening everything into one noisy stream.

Today, reasoning is mixed into answer text while every tool call becomes its
own card. The actual answer gets buried, and the transcript feels busier as
harnesses expose richer event streams.

```
agent events             transcript
────────────             ──────────
reasoning deltas   ───►  Thinking   (collapsible)
tool calls/results ───►  Activity   (grouped)
answer deltas      ───►  Answer     (primary)
```

The transcript should remain harness-neutral: providers emit common events,
and one shared presentation turns them into calm, contiguous segments.
Thinking collapses once the answer begins; consecutive tools form one activity
group; the answer remains visually primary.

This direction does not require harness-specific views or a redesign of stored
chat history. Compatibility with existing sessions remains a constraint for
the later design.
````
