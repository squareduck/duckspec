# Change review record

A review is an advisory critique of a change, captured as a document and kept as a
permanent part of that change's record.

## What a review is

A review is a document-schema artifact — the same shape as a proposal or a design: an H1
title, a summary, and a body. Its content is the judgment that static verification can't
make: `ds audit` and `ds check` prove a change is well-*formed*, while a review judges
whether it is well-*conceived* and well-*made*. The body is conventional, not enforced
beyond the document rules, but `ds schema review` prescribes its shape — findings tagged
by lens and severity, any open questions, and an aggregate verdict.

A review reads *down* a change's chain — `proposal → design → caps → code` — to the
deepest artifact that exists, so it applies at any stage, from a proposal-only plan to a
fully-implemented change. It judges along three lenses: **soundness** (is each artifact
right on its own terms?), **fidelity** (does each layer faithfully realize the one above
it?), and **quality** (is the work simple, idiomatic, well-made?).

Reviews are advisory. They record judgment a reader or agent can act on, but they decide
nothing on their own — nothing in the system blocks or gates on a review.

## Where reviews live

Each change owns a `reviews/` directory. Reviews are files directly inside it, named
`NN-<slug>.md`, where `NN` is a two-digit sequence number and `<slug>` is a kebab-case
label derived from the review's title:

```
changes/<name>/
  reviews/
    01-pre-implementation.md
    02-post-implementation.md
```

A review is recognized purely by this location — any markdown file directly under a
change's `reviews/` directory is a review and is validated as a document.

## Numbering and order

Reviews form an append-only sequence. Creating a review assigns it the next number — one
greater than the highest review already present — so the directory reads as a
chronological log. A change's first review is `01`.

Reviews are never renumbered or inserted between existing entries, and two reviews in the
same change cannot share a slug. The result is a stable, ordered history: a review's
number never changes once assigned.

## Lifecycle and archival

Reviews are written at any point in a change's life — a critique of the design before
implementation, a critique of the code after, or any number in between. Because they live
inside the change directory, reviews travel with the change when it is archived: the full
sequence of reviews is preserved alongside the rest of the change's record, so the
critique history outlives the work.
