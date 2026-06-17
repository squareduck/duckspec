# Validated delta merge

The blessed way to apply a delta to an artifact and get back a result that is guaranteed
to satisfy the artifact's schema. It wraps the raw merge with a validation pass so every
consumer — status, audit, archive — handles merge results the same way and gets the same
typed errors.

A raw merge operates on the generic heading tree and is content-agnostic: it can produce
text that is structurally merged but no longer a valid spec or doc. Validated merge closes
that gap by re-parsing the merged text with the parser for the artifact's kind, so a
successful result is always a parsed, schema-valid artifact — never just a string the
caller must re-check.

## Outcomes

A merge produces one of two successful outcomes or one of two errors.

```text
| Outcome   | Meaning                                              |
| --------- | ---------------------------------------------------- |
| Updated   | the delta applied; carries rendered text + artifact  |
| Deleted   | the delta removed the whole artifact (— on the H1)   |
| Merge err | the delta did not apply to its source                |
| Parse err | the merged text did not satisfy the artifact's schema |
```

The `Updated` outcome carries both the rendered markdown (what gets written to disk) and
the already-parsed artifact, so a caller that needs the parsed form does not re-parse.

## Kind-matched validation

Validation uses the parser that matches the artifact being merged.

```text
| Artifact kind | Validated with    |
| ------------- | ----------------- |
| capability spec | spec parser     |
| capability doc  | document parser |
```

Specs and docs have different shapes — a spec requires `Requirement:` and `Scenario:`
headings, while a doc is a free heading tree — so each must be validated with its own
parser for the result to be meaningful.

## Error classification

The two error categories separate *where* a merge went wrong, which a caller needs in
order to report it usefully:

- A **merge error** means the delta itself does not fit the source — for instance it
  targets a heading that is not there. The delta is at fault.

- A **parse error** means the delta applied but the combined result is no longer a valid
  artifact. The merged content is at fault.

A failure may carry several underlying errors. When rendered, it collapses to a single
line: the first error's message followed by a count of how many more there are, so a
multi-error failure stays readable on one line.
