# Clearer spec delta schemas - Design

Clarify stock authoring schemas (and a thin template pointer) so delta file shape for
heavy remove/replace matches merge semantics — no engine change.

## Approach

Content-only change to embedded stock files under `crates/duckspec/content/`. Ownership
stays the codex split: **schemas own file grammar and quality judgment; the `/ds-spec`
template owns conversation process.**

```
proposal intent
      │
      ▼
┌─────────────────┐     ┌──────────────────┐
│  schema files   │◄────│  template/spec   │
│  (how to write) │     │  (how to talk)   │
└────────┬────────┘     └──────────────────┘
         │ load via `ds schema` / `ds template`
         ▼
   agents authoring deltas
```

No parser, merge, or marker set changes. Semantics already in
`crates/duckpond/src/merge.rs` become **explicit authoring rules**, not new behavior.

## Spec delta schema

Path: `crates/duckspec/content/schemas/spec-delta.md`

Keep skeleton: Structure → Markers → Rules → Quality → Formatting → Example.

**Markers table** — expand beyond one-line ops so each marker states:

```
| Marker | Body | Nested headings | Effect on source |
| --- | --- | --- | --- |
| `@` | empty keep / non-empty replace | operations (each marked) | surgical children |
| `~` | always replace | content (no markers) | wipe children; re-list full new subtree |
| `+` | new body | content (no markers) | insert |
| `-` | must empty | none | delete header + subtree |
| `=` | new-name line only | none | rename; later ops use new name |
```

**Rules** — add only what is currently implicit:

- `@` with empty body preserves body; non-empty replaces body only (children still via
  child ops)

- Under `@`, every child heading is an operation marker; under `+`/`~`, nested headings
  are plain content (e.g. `### Scenario: …` with **no** `+`/`-`/`~`)

- Scenario body rewrite: `~` on H3 under `@` parent (never `@` on H3)

- Point at `ds schema spec` for GWT / test markers on all authored bodies

**Structure skeleton** — optional one-line note under the fence: “under `+`/`~`, nested
headings have no markers; under `@`, they do.” Avoid two full skeletons.

**Quality** — keep altitude short; replace vague “prefer `@` + `+`” with a decision rule:

```
| Situation | Prefer |
| --- | --- |
| Few children add/remove/edit | `@` parent + child ops |
| Most of a requirement’s scenarios rewritten | `~` requirement + full new body/scenarios as content |
| Norm prose only, scenarios stay | `@` with body text, no H3s |
| Rename needed | `=` then `@`/`~` under the **new** name |
```

Stable titles / cold-reader / no restatement-only rewrites stay.

**Example** — one multi-marker block (codex guidance), not a catalog. Cover:

1. `=` rename (name line only)
2. `-` remove a requirement
3. `@` with empty-or-body + `### -` / `### +` scenario ops
4. `+` requirement with **unmarked** `### Scenario` content children

Fix current bug: example today has `### + Scenario` under `## + Requirement` (invalid
content-child markers). Align with merge fixtures (`mixed_operations_delta.md`,
`replace_requirement_delta.md`).

## Doc delta schema

Path: `crates/duckspec/content/schemas/doc-delta.md`

Parallel to spec-delta:

- Same expanded marker semantics (doc sections instead of Requirement/Scenario)

- Rules: keep “same mechanics as spec-delta” for shared grammar; state body/children rules
  once here so doc-only loads still work

- Quality: same lightest-touch vs `~` section decision; cold reader; keep pace with spec

- Example: multi-marker (`-` section, `~` section, `+` section) instead of only `~` + `+`

## Base spec and doc schemas

Paths: `crates/duckspec/content/schemas/spec.md`, `doc.md`

Minimal cross-link only — **no** delta marker grammar:

- One Rules or post-Rules sentence: bodies under deltas and merged results must still
  satisfy this schema; delta shape is `ds schema spec-delta` / `doc-delta`

- Do not expand Structure or restate markers

## Spec stage template

Path: `crates/duckspec/content/templates/spec.md`

Thin process-only edit on the “On disk after each confirm” bullets:

- **Today (duplicates Quality):**
  `lightest-touch … (prefer @ + + over rewrites; prefer body edit over rename…)`

- **After:** point at schemas for *how* the delta is written; keep process *when* (UPDATE
  → `.delta.md`, REMOVE → H1 `-`, load schemas before draft)

```
Update: `spec.delta.md` / `doc.delta.md` per `ds schema spec-delta` /
`doc-delta` (marker choice and lightest touch live there)
```

No marker tables, no multi-marker example, no `@` body-preserve rules in the template.

## Impact

- Stock content only; `ds schema` / `ds template` serve new text after rebuild or
  workspace run

- No duckpond API, parse, or merge change

- No capability behavior change under `duckspec/caps/` unless we later choose to document
  stock-content text (not required for this approach)

- Existing unit test that schemas exist by H1 title remains valid; no known golden
  snapshots of full schema bodies

## Decisions

- **One multi-marker example, not two** — matches `template-and-schema-authoring` codex;
  avoids edge-case catalog. Heavy remove/replace lives in that one example plus the
  Quality table.

- **Expand Markers table rather than a long freeform Semantics section** — scannable;
  Rules stay mechanical bullets.

- **Move lightest-touch / stable-title judgment out of the template** — ends split brain;
  template only names which schema to load for UPDATE expansion.

- **No engine changes** — teach existing merge semantics; friendlier errors are out of
  scope unless schemas still fail agents later.

## Risks

- **Schemas get longer / overload progressive load** → keep Structure compact; put
  judgment only in a short Quality table; one example.

- **Agents still only skim the Example** → multi-marker example must include the
  failure-prone patterns (content children under `+`, scenario `-`/`+` under `@`,
  requirement `~`).

- **doc-delta “see spec-delta” still leaves gaps if only doc schema is loaded** → restate
  body/children semantics in doc-delta Rules briefly, not only by reference.
