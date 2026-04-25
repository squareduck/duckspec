# Pin parse spec

Wire `@spec` backlinks for the 19 `parse/spec` scenarios. 5 are backed by existing
happy-path fixtures; 14 require new error fixtures under `tests/fixtures/spec/errors/`
plus matching `expect_err` snapshot tests in `tests/parse_spec.rs`.

## Prerequisites

- [ ] @step tighten-parse-marked-heading

## Context

For each new error fixture, create a minimal `.md` file under
`crates/duckpond/tests/fixtures/spec/errors/<variant>.md` that triggers the target
`ParseError` variant, then add a corresponding test in
`crates/duckpond/tests/parse_spec.rs`:

```rust
#[test]
fn err_<variant>() {
    let errors = parse_fixture("errors/<variant>.md").expect_err("should fail");
    insta::assert_debug_snapshot!(errors);
}
```

Add a `// @spec parse/spec <Requirement>: <Scenario>` comment above each test (both new
error tests and existing happy-path tests). Run `cargo insta review` to accept new
snapshots, then `ds sync` and `ds audit`.

The 14 new fixture files:

```
tests/fixtures/spec/errors/
├── content_before_h1.md           ← ContentBeforeH1
├── missing_h1.md                  ← MissingH1 (empty file)
├── missing_summary.md             ← MissingSummary (H1 only)
├── heading_too_deep.md            ← HeadingTooDeep (contains H4)
├── invalid_requirement_prefix.md  ← InvalidRequirementPrefix
├── requirement_name_colon.md      ← RequirementNameColon
├── empty_requirement.md           ← EmptyRequirement
├── invalid_scenario_prefix.md     ← InvalidScenarioPrefix
├── missing_when_or_then.md        ← MissingWhen + MissingThen
├── unexpected_scenario_content.md ← UnexpectedScenarioContent
├── gwt_out_of_order.md            ← GwtClauseOutOfOrder × 3 branches
├── invalid_gwt_keyword.md         ← InvalidGwtKeyword
├── invalid_test_marker.md         ← InvalidTestMarker
└── unresolved_test_marker.md      ← UnresolvedTestMarker
```

## Tasks

- [x] 1. @spec parse/spec Spec document structure: Minimal spec parses successfully

- [x] 2. @spec parse/spec Spec document structure: Spec with multi-paragraph description parses successfully

- [x] 3. @spec parse/spec Spec document structure: Content before H1 raises ContentBeforeH1

- [x] 4. @spec parse/spec Spec document structure: Missing H1 raises MissingH1

- [x] 5. @spec parse/spec Spec document structure: Missing summary raises MissingSummary

- [x] 6. @spec parse/spec Spec document structure: Headings deeper than H3 raise HeadingTooDeep

- [x] 7. @spec parse/spec Requirement section structure: Spec with multiple requirements parses successfully

- [x] 8. @spec parse/spec Requirement section structure: H2 without 'Requirement: ' prefix raises InvalidRequirementPrefix

- [x] 9. @spec parse/spec Requirement section structure: Requirement name containing a colon raises RequirementNameColon

- [x] 10. @spec parse/spec Requirement section structure: Requirement with neither prose nor scenarios raises EmptyRequirement

- [x] 11. @spec parse/spec Scenario section structure: H3 without 'Scenario: ' prefix raises InvalidScenarioPrefix

- [x] 12. @spec parse/spec Scenario section structure: Scenario missing WHEN or THEN raises MissingWhen and MissingThen

- [x] 13. @spec parse/spec Scenario section structure: Non-GWT content inside scenario body raises UnexpectedScenarioContent

- [x] 14. @spec parse/spec GWT phase machine: Out-of-order GWT clauses raise GwtClauseOutOfOrder

- [x] 15. @spec parse/spec GWT phase machine: Unrecognized GWT keyword raises InvalidGwtKeyword

- [x] 16. @spec parse/spec Test markers: Test marker inherits from requirement to scenario

- [x] 17. @spec parse/spec Test markers: Test code marker carries backlinks

- [x] 18. @spec parse/spec Test markers: Unrecognized test marker prefix raises InvalidTestMarker

- [x] 19. @spec parse/spec Test markers: Scenario with no marker and requirement with no marker raises UnresolvedTestMarker

## Outcomes

Steps 04, 05, and 06 still have the wrapped-task bug that step 02 documented —
`ds audit` shows their `@spec` task lines being truncated mid-scenario-name. The next
`/ds-apply` session will need to rewrite each of those step files with each task on a
single line before implementing them, same fix applied here.
