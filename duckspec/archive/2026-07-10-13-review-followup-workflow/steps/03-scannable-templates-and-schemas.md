# Scannable templates and schemas

Ship peer followup content and rewrite review schema/template for scannable summary
tables, structured findings, and a shared document-first write gate.

## Prerequisites

- [x] @step kind-prefixed-critique-create
- [x] @step lifecycle-dual-critique-chrome

## Tasks

- [x] 1. Rewrite `crates/duckspec/content/schemas/review.md` with Summary table +
         structured Findings / Verdict per design; keep stock schema sections (Structure,
         Severity, Rules, Quality, Formatting, Example)

- [x] 2. Add `crates/duckspec/content/schemas/followup.md` as a peer (Issues / Outcome
         naming allowed) with the same scannable spine

- [x] 3. Rewrite `crates/duckspec/content/templates/review.md` and add
         `templates/followup.md`: stock template sections, voice split (firm critique vs
         explore-like), create kind-prefixed files via `ds create review` /
         `ds create followup`, document-first write gate (critique file only; plan/code
         only on explicit post-doc ask or later stages); chat triage matches file Summary
         table

- [x] 4. Add command wrappers `crates/duckspec/content/commands/claude/ds-followup.md` and
         `opencode/ds-followup.md` (same one-liner pattern as `ds-review.md`); ensure
         `ds template followup` resolves

- [x] 5. Add `/ds-followup` to the lifecycle skill list in
         `crates/duckchat/src/reply_suggest.rs` (and any related tests); run
         `cargo test -p duckchat` / `cargo test -p duckspec` as needed for template
         discovery
