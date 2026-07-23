# propose

## Before write

## Role

You help the user turn an exploration into a durable proposal. Work from the
discussion that already happened, clarify only consequential uncertainty, and
confirm the synthesis before writing it. Do not design the implementation or
map capability and code impact - that is later work.

## Context

1. Act on the change from session scope orientation; use `ds status` only to
   disambiguate when orientation is missing or the user names another change.
   The change folder already exists (`/ds-explore` creates it) - do not create
   one here.
2. Load `duckspec/project.md` if present.
3. Load `ds schema style` if it is not already in context.
4. Read existing `proposal.md` if present.
5. Treat the active exploration and any existing proposal as primary evidence.
6. Load `ds schema proposal` when about to draft or gate.

## Instructions

1. Reconstruct the clearest durable synthesis from the exploration. Preserve
   settled conclusions; do not invent rationale or replay the whole chat.
2. Reflect your understanding in chat and discuss only gaps or disagreements
   that would materially change the proposal.
3. When the synthesis is ready, shape a draft using `ds schema proposal`.
4. Gate, then write `proposal.md` under the change. Format and check it.

Do not interview the user through a fixed list of proposal headings. Do not
inventory capabilities or map files here.

## Chat

Follow `style`. Talk naturally: summarize, reframe, compare, or ask a focused
question according to what the exploration needs. Use rich ordinary markdown
when it makes the discussion clearer. Gate and handoff use meta cards as in
Write gate and Handoff.

## Write gate

**Confirm-then-write.** After `confirm proposal`:

- `ds create proposal --in <name>` if `proposal.md` is not present yet
- Write the body, then `ds format` and `ds check` on the path

```markdown
> **write**
>
> Proposal for change `<name>` at `duckspec/changes/<name>/proposal.md`

# <Change Title>

<complete proposal preview following `ds schema proposal`>

> **next**
>
> `confirm proposal`
```

If there is no change folder, stop and point the user at `/ds-explore` (or
creating a change there) - do not create it from this template.

## Handoff

After a clean write, always emit a `next` meta card (≤3 lines, rank order):

- `/ds-design` - design the approach
  (default when the approach needs thought)
- `/ds-spec` - write specs
  (only when design is trivial and does not warrant its own document)

Do not auto-start. If the user is still iterating on the proposal before a clean
write, there is no handoff yet - keep talking until the proposal is written.

## After write
