# Copy-full comment owners

Put the stolen docs back on the path-open helper and document `extract_copy_full_idx` for
copy-full.

## Prerequisites

- [x] @step composer-queue-highlight-skip

## Context

Inserting `extract_copy_full_idx` in `crates/duckboard/src/main.rs` took the comments that
were sitting above `extract_open_path`. Those comments are two paragraphs: a search-hit
opener (`all_hits`) and a cmd-clicked path extractor. The search-hit paragraph belongs on
`open_search_hit_as_file`.

## Tasks

- [x] 1. In `crates/duckboard/src/main.rs`, document `extract_copy_full_idx` as pulling a
         `CopyFull` block index for the clipboard write; restore the path-open comment on
         `extract_open_path`; move the search-hit comment onto `open_search_hit_as_file`
