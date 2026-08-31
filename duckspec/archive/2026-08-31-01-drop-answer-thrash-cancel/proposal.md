# Drop answer-thrash cancel

Stop force-cancelling a turn when the assistant rewrites its live answer after thought.
Keep replacing the draft so the transcript still shows one current answer.

Duckboard currently treats answer-after-thought as a small client budget (`N=1`). The
first rewrite is allowed; the next one cancels the in-flight turn, keeps the last draft,
and shows a stop notice. The detector counts replacements. It does not compare whether the
new text is actually a repeat.

That cancel was added for write-gate turns that thought, emitted a full answer, thought
again, and re-emitted a lightly reworded gate — stacking bubbles and hanging for minutes
with no tools. Live draft replace already prevents the stacked bubbles. The cancel was
meant to stop the hang and token burn.

On Grok, the cancel is a false positive. After `spawn_subagent` (a tool use, which resets
the counter), the parent typically thinks, emits a status answer, thinks again, and emits
another. The second rewrite trips the budget and kills the Grok child, so every in-flight
subagent dies with it. The wasted tokens are the children’s work, not a rewrite loop.

```
keep
  answer ──reasoning──► draft stays uncommitted
         ──answer───► live draft replaced (prior body discarded)
         ──tool──────► draft committed; later answer is a new span
         ──turn end──► draft committed

drop
  over-budget rewrite ──► cancel turn · stop notice · kill agent child
```

The change is harness-neutral. Grok hits it because it reasons and streams parent status
while children run; Claude write-gates were the original target. Do not special-case Grok,
raise the budget, or remove draft replace.
