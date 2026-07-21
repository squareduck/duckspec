# Clearer spec delta schemas

Make stock spec/doc and delta schemas teach heavy remove and replace so agents author
correct deltas without restating file grammar in templates.

## Motivation

Agents often get lost writing spec and doc deltas when many scenarios must be removed or
rewritten. Merge behavior is consistent (`@` vs `~` vs content children under `+`/`~`),
but the authoring schemas under-teach those cases: the marker table is one-liners,
"optionally replace body" never says empty preserves, and the worked example is only light
`@` + `+` add paths. Real changes already do bulk `-`, scenario swap, and
whole-requirement `~`; agents invent wrong shapes instead of following a clear rule.

Why now: delta authoring is load-bearing for every UPDATE capability, and template vs
schema ownership is already clear in codex — the fix is schema altitude, not more process
prose.

## Intent

- An agent loading `spec-delta` / `doc-delta` can choose the right marker for surgical
  edit, bulk scenario removal, full requirement rewrite, and rename-then-edit without
  guessing

- `@` body preserve vs replace, and content vs operations children under `+`/`~` vs `@`,
  are stated as rules (not implied by one happy-path example)

- Quality gives a short decision rule for lightest touch vs parent `~` when most of a
  subtree changes

- The canonical example is multi-marker (remove/replace/add), matching existing authoring
  guidance for deltas

- Base `spec` / `doc` schemas only make it obvious that merged and delta-authored bodies
  still must satisfy those schemas

- `/ds-spec` may point at those schemas for how files are written; it does not duplicate
  marker grammar or delta examples

## Non-goals

- Changing merge, parse, or marker semantics

- New delta markers or a different delta format

- Expanding the schema example into a full edge-case catalog

- Putting file-shape rules in the template (template stays conversation: map, outline,
  gates)

- Broader template redesign beyond clarity pointers to schemas
