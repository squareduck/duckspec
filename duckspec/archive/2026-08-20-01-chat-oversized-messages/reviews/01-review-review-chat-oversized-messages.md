# Review: Chat oversized messages

Implementation matches the design for the size gate, user-card prefix, copy-full, and
composer/queue highlight skip. One comment-only fix remains before archive.

## Scope

Reviewed the change proposal, design, `chat/oversized-messages` spec/doc, stream-ui and
transcript doc deltas, all three steps, duckboard source and `@spec` tests, and the
working-copy diff. `ds check` and `ds audit chat-oversized-messages` were clean. No prior
review file.

## Summary

```
| # | finding | resolution | → next |
| --- | --- | --- | --- |
| 1 | `extract_copy_full_idx` inherited the wrong docs | Restore path-open docs; document copy-full | `/ds-step` |
```

## Findings

### 1. `extract_copy_full_idx` inherited the wrong docs

**Where:** `crates/duckboard/src/main.rs:4943`

**Evidence:** Inserting `extract_copy_full_idx` took the comments that belonged to
`extract_open_path`. The new helper is documented as opening a search hit and pulling a
cmd-clicked path (`Returns (path, 1-based line)`). `extract_open_path` has no doc comment.

**Impact:** The comments describe a different function. Later readers will trust them and
mis-wire copy-full or path-open.

**Discussion:** Leaving the comments would ship a known mismatch for no gain. Restoring
the path-open docs and adding a short copy-full comment is comment-only; design and specs
stay valid.

**Resolution:** Put the existing docs back on `extract_open_path`. Document
`extract_copy_full_idx` as pulling a `CopyFull` block index for the clipboard write.

**Next:** `/ds-step` - comment-only fix on those two functions.

## Resolved concerns

The working-copy edit to `crates/duckspec/content/schemas/spec.md` (no visual tests in
specs or docs) is unrelated to oversized chat and was kept in this change by agreement.

Proposal scope included Answer/Thinking truncation; the design rejected that and kept
agent-echo hitch as known. Implementation follows the design.

## Outcome

Not ready to archive. Design and specs stand; fix the misplaced comments, then archive.
