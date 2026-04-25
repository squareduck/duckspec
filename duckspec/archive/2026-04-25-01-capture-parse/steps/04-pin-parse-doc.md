# Pin parse doc

Wire `@spec` backlinks for the 4 `parse/doc` scenarios. 3 are backed by existing
happy-path fixtures; 1 requires a new `deep_recursion.md` fixture.

## Prerequisites

- [ ] @step tighten-parse-marked-heading

## Context

Add a new fixture `crates/duckpond/tests/fixtures/doc/deep_recursion.md` containing an H1
+ summary + an H2→H3→H4→H5 heading chain with body content at each level. Add a matching
test in `crates/duckpond/tests/parse_doc.rs` that loads the fixture and snapshots the
resulting `Document`.

For each test (existing and new), add a `// @spec parse/doc <Requirement>: <Scenario>`
comment above the `#[test]` line. Run `ds sync` and `ds audit` after.

## Tasks

- [x] 1. @spec parse/doc Document structure: Minimal document parses successfully

- [x] 2. @spec parse/doc Section tree: Document with sibling H2 sections parses into a flat section list

- [x] 3. @spec parse/doc Section tree: Document with nested headings produces a parent-child section tree

- [x] 4. @spec parse/doc Section tree: Section tree captures headings nested four levels deep
