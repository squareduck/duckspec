# Concrete gate tokens

Gate chips send decision-named tokens instead of bare `confirm` / `reject`, so a confirm
always names what was accepted — especially across multi-gate `/ds-spec` passes after
thrash or cancel.

## Motivation

Bare `confirm` is phase-ambiguous. On `/ds-spec`, chained pure-text gates plus answer
thrash make that fatal: the user confirms a draft the agent may not remember, and the
agent re-answers the wrong gate. Template reshapes (split outline/write turns,
map-plus-first-outline) did not stop the loop; cancel-resync fixes transcript divergence
but still leaves a generic token that does not name the decision.

Why now: resync is already shipped, reshapes have failed twice, and multi-word send tokens
already work (meta-cards take the full first code span). Concrete tokens are the remaining
protocol fix, and they should apply consistently across write gates — not only in
`/ds-spec`.

## Intent

- Every write-gate send token names the decision (e.g. `confirm proposal`, `confirm map`,
  `confirm <path>`), not a generic `confirm` / `reject`

- `/ds-spec` uses a separate map stage again, keeps REMOVE vocabulary, and uses path- or
  stage-qualified confirms for map, each capability, and remove

- Decision tokens stand alone (no reason text beside them); slash-command handoffs keep
  short UI reasons

- Style (and templates that mirror it) teach concrete tokens as the default; bare
  `confirm` / `reject` are not the stock pattern

- After cancel or thrash, confirming still carries which gate was accepted in the user
  message itself

## Non-goals

- Fixing grok answer-rewriting or changing thrash N=1
- Distinct confirm tokens as a substitute for cancel-resync (keep both)
- Changing meta-card syntax, card kinds, or the parser (optional reasons stay supported)
- Harness-level loop fingerprinting or tool-call debounce
- Redesigning chip UI to display reasons (reasons remain non-send text only)
