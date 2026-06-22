# Advisory review stage

Add a `/ds-review` stage — a fresh-session, advisory reviewer that critiques a change
against its contract and the diff, writing a persistent, sequentially-numbered review
record that can spawn fix-steps.

## Motivation

`ds check` and `ds audit` mechanize *drift* detection — backlinks resolve, `test: code`
scenarios are covered — but nothing in duckspec judges *implementation quality*: bugs,
code smells, architectural drift, or a test that links back to a scenario yet asserts the
wrong thing. There is a gap between "mechanically clean" and "actually good", and no place
where that judgment is captured.

An advisory review stage closes the loop. And because reviews live in the change folder,
they ride into `archive/` with the change — so the critique history becomes part of the
permanent record, not chat that evaporates.

## Scope

```
caps/
├── review/                   ← NEW
│   ├── spec.md
│   └── doc.md
└── session/
    └── scope/
        └── spec.md           (modified — orientation surfaces current review)
```

The change-folder model also gains one location:

```
changes/<name>/
  proposal.md · design.md · caps/ · steps/
  reviews/                    ← NEW
    01-pre-implementation.md
    02-post-implementation.md
```

### New capabilities

- `review` — reviews as an advisory, `doc`-schema artifact under
  `changes/<name>/reviews/NN-<slug>.md`: `ds create review` assigns the next sequential
  number, the "current review" is the highest-numbered one, and a review never alters its
  change's phase or suggested next stage.

### Modified capabilities

- `session/scope` — session orientation gains a "current review" field (the highest-`NN`
  review of the active change), and phase derivation explicitly ignores `reviews/`.

### Out of scope

- Typed `@review` references resolved by audit — v1 cites the source review via a prose
  path in a generated step's `## Context` section.

- Review as a *gate* — it stays purely advisory; archive remains governed only by the
  mechanical scoped audit.

- A structured, machine-readable findings schema — reviews stay `doc`-schema; their
  structure lives in the `/ds-review` template, not the parser.

- Auto-running step generation — the step-generating second movement of `/ds-review` is
  always user-initiated.

## Impact

Purely additive. New `reviews/` location in the change-folder model; new
`ds create review` command and a `ds status` review listing; new `review` template,
`ds template review` rendering, and `/ds-review` skill. The `session/scope` change ships
as a `spec.delta`, exercising the delta-merge and archive-backlink-guard paths. No
breaking changes.
