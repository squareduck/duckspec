# Change review record

A review is an advisory critique of a change, captured as a document and kept as a
permanent part of that change's record.

## What a review is

A review is a document-schema artifact — the same shape as a proposal or a design: an H1
title, a summary, and freeform body. Its content is a critique of the change it belongs
to: findings, their severity, and recommended actions. The document structure is
conventional, not enforced by a schema beyond the document rules; a review is free to
organize its findings however suits the critique.

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
