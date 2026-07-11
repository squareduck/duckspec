# Turn answer replace

Bound mid-turn answer thrash: one replaceable draft per thought span, a hard stop when the
model keeps rewriting, shared write-gate “emit once” discipline, and slightly faded
Thinking text.

## Motivation

On pure-chat wait turns (write-gates for proposal, design, and especially capability
outlines), the model often finishes a full answer, thinks again, and re-emits a lightly
reworded full answer many times before the turn ends. Duckboard’s kind-switch flush turns
each rewrite into its own message, so the UI stacks near-duplicate confirm surfaces.
Worse, the turn can run for minutes with no tools and no stop — replace-alone would clean
the transcript but still leave the hang and token burn.

Why now: this fires consistently on confirm-then-outline and has also hit proposal/design
gates in the same change; it is the highest-friction main-turn failure after the
empty-session work.

## Intent

- Within one turn, thought interleaved with answer does not leave multiple full answers on
  disk or as stacked bubbles — only the current draft, then one committed answer when that
  span ends

- Live answer text is replaced when a new answer stream starts after thought

- Answer text that ends because tools start is still committed; a later answer after tools
  is a new span

- After two answer-after-thought replacements in one turn without a tool boundary, the
  client cancels the turn, keeps the last draft, and shows a short stop notice (no
  automatic re-run)

- Shared write-gate style tells agents: after emitting a gate that awaits confirm, end the
  turn — do not re-emit or polish that gate in the same turn (all providers, all stages
  that use write gates)

- Thinking body copy is slightly more faded than normal answer text — still legible,
  clearly secondary

## Non-goals

- Making the model never regenerate (no harness “one final answer” magic)
- Similarity detection of rewrite vs continuation
- Provider-specific or Grok-only harness coalesce
- Empty-session lifecycle, promotion focus, meta-card parsing, oneshot hints
- Redesigning full Thinking/Answer chrome beyond a mild body fade
- Per-stage template essays; only a minimal shared write-gate rule
