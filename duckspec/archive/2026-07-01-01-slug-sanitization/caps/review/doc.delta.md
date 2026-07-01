# @ Change review record

## ~ Where reviews live

Each change owns a `reviews/` directory. Reviews are files directly inside it, named
`NN-<slug>.md`, where `NN` is a two-digit sequence number and `<slug>` is a kebab-case
label derived from the review's title by the canonical slug rule (the `slug` capability):

```
changes/<name>/
  reviews/
    01-pre-implementation.md
    02-post-implementation.md
```

A title with no alphanumeric characters would yield an empty slug, so creating a review
from such a title is rejected rather than writing an unnamed file.

A review is recognized purely by this location — any markdown file directly under a
change's `reviews/` directory is a review and is validated as a document.
