# Workflow slash order

Make `/ds-*` slash completion follow the default duckspec workflow order, with short stage
descriptions from command frontmatter, so the popup reads as a map of the process instead
of an alphabetical skill dump.

## Motivation

When users type `/ds` in duckboard, workflow commands appear in discovery/alpha order and
most rows have empty descriptions — the thin command stubs never declared frontmatter. The
list does not teach stage order, so newcomers and occasional users cannot see “what comes
when” while browsing.

Why now: slash completion already kind-tags Workflow (`ds-*`) vs System vs Agent; the next
minimal gap is order and copy *inside* Workflow, without new chrome, LLM suggestions, or a
second guidance system.

## Intent

- Typing `/` (or filtering to `ds`) lists known duckspec workflow commands in default
  lifecycle order (explore → propose → design → spec → step → apply → archive), with
  optional/side stages interleaved by agreed order keys

- Each workflow row shows a short human description of what that stage does

- Order and description come from command-file frontmatter (the files `ds init` installs),
  not from a parallel hard-coded product narrative in the UI

- Fuzzy match score stays primary when the query is non-empty; workflow order is the
  tie-break / empty-query ordering for Workflow entries

- Unrecognized `ds-*` names (no order metadata) still appear, after ordered ones

- Side / optional stages are unnumbered in the UI — sort only, no stage-index chrome

## Non-goals

- Stage number badges, progress rails, or “you are here” highlighting in the completion
  popup

- Mid-line `/` completion (slash only at start of input)

- LLM-generated next-stage suggestions or changing empty-composer / `next` meta-card
  authority

- Full help modal, keybind browser, or workflow tutorial beyond what local `/help` already
  lists

- Commit / push / test / doc as first-class duckspec stages

- Redesigning non-`ds` skill discovery or Agent ordering beyond existing kind tie-breaks
