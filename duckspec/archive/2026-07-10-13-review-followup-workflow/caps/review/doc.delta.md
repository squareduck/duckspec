# @ Change review record

A critique record is an advisory document kept as a permanent part of a change's history.
Two create kinds share one append-only `reviews/` log: **review** (agent-led judgment) and
**followup** (user-led course correction). Both are recognized as review artifacts and
validated as documents.

## ~ What a review is

A critique record is a document-schema artifact — the same shape as a proposal or a
design: an H1 title, a summary, and a body. Its content is the judgment that static
verification can't make: `ds audit` and `ds check` prove a change is well-*formed*, while
a review or followup records whether the work is well-*conceived* and well-*made*, and
what should happen next in the plan chain.

The body is conventional, not enforced beyond the document rules. Agent schemas prescribe
a scannable shape — a summary table of issues, structured detail headings, and an
aggregate verdict or outcome — but `ds check` only validates document rules.

A **review** is typically agent-led: it reads *down* a change's chain —
`proposal → design → caps → code` — to the deepest artifact that exists and judges along
three lenses: **soundness**, **fidelity**, and **quality**. A **followup** is typically
user-led: the same history log and lenses vocabulary, driven by conversation rather than a
solo scan. Either kind records issues and recommended next stages; neither amends
proposal, design, caps, or steps as part of its normal write gate, and neither implements
product code. Plan or code changes follow via later stages or an explicit post-document
request from the user.

Critique records are advisory. They record judgment a reader or agent can act on, but they
decide nothing on their own — nothing in the system blocks or gates on a review file's
presence beyond lifecycle chrome that *offers* rework and further critique.

## ~ Where reviews live

Each change owns a `reviews/` directory. Critique files sit directly inside it, named
`NN-<slug>.md`, where `NN` is a two-digit sequence number and `<slug>` is a kebab-case
label. On create, the slug is the kind prefix plus the title slug from the canonical slug
rule (the `slug` capability):

```
changes/<name>/
  reviews/
    01-review-pre-implementation.md
    02-followup-collapse-policy.md
    03-review-post-implementation.md
```

```
| Kind     | Filename slug shape        |
|----------|----------------------------|
| review   | `review-<title-slug>`      |
| followup | `followup-<title-slug>`    |
```

A title with no alphanumeric characters would yield an empty title slug, so create is
rejected rather than writing an unnamed file.

Recognition is by location only — any markdown file directly under a change's `reviews/`
directory is a review artifact and is validated as a document. Legacy files without a kind
prefix remain recognized; new creates always write a kind-prefixed slug.

## ~ Numbering and order

Review and followup creates share one append-only sequence. Creating either assigns the
next number — one greater than the highest critique file already present — so the
directory reads as a chronological log. A change's first file is `01`.

Files are never renumbered or inserted between existing entries. Two files in the same
change cannot share the same full slug (kind prefix included), so `review-post-impl` and
`followup-post-impl` may both exist. The result is a stable, ordered history: a file's
number never changes once assigned.

## ~ Lifecycle and archival

Critique records are written at any point in a change's life — a design critique before
implementation, a mid-flight followup that records course correction, a
post-implementation review, or any number of re-passes. Because they live inside the
change directory, they travel with the change when it is archived: the full sequence is
preserved alongside the rest of the change's record, so the critique history outlives the
work.
