# Unify slug generation

Replace the divergent title-to-slug functions with one canonical rule that maps every
non-alphanumeric character to `-`, so `ds create` stops producing filenames like
`NN-post-implementation:-soundness-&-fidelity.md`.

## Motivation

`ds create review` (and `ds create step`) turn a human title into an `NN-<slug>.md`
filename via `slugify`, but the `slugify` used for filename creation only lowercases and
collapses whitespace — it leaves every other character intact. A review titled
"Post-implementation: soundness & fidelity" becomes
`NN-post-implementation:-soundness-&-fidelity.md`: colons, ampersands, and slashes leak
straight into the filename.

The rule is duplicated and inconsistent — three `slugify` functions exist:

- **`plan.rs`** (filename creation): lowercase + collapse whitespace only; leaks all other
  punctuation. This is the bug.

- **`artifact::step::slugify`** (step slug derivation/validation): correct — maps every
  non-alphanumeric to `-`, Unicode-aware. Used only on the read/validate side.

- **`duckboard::idea_store::slugify`** (idea filenames): also maps non-alphanumeric to
  `-`, but ASCII-only and with an `"idea"` fallback for empty results.

The consequences: **reviews** get ugly-but-valid names (no slug-mismatch check catches
them); **steps** are worse — the creation slug and the validation slug disagree, so a
punctuated step title produces a file that immediately fails `ds check` with a slug
mismatch; and **ideas** silently drop non-Latin characters.

One rule, used everywhere, fixes all three. This is a small, contained cleanup of a
visible defect and two latent inconsistencies.

## Scope

```
caps/
├── slug/                 ← NEW  (canonical title→slug rule)
├── parse/
│   └── step/  (modified — slug derivation relocates to the canonical rule;
│              no behavior change)
└── review/    (modified — creation honors the canonical rule; empty-slug
               title rejected)

+ duckboard idea filenames adopt the canonical rule (Unicode-aware).
```

### New capabilities

- `slug` — the canonical title-to-slug rule and single source of truth: lowercase; keep
  Unicode alphanumerics; map every run of non-alphanumeric characters to a single `-`;
  trim leading and trailing `-`. The rule may yield an empty string (all non-alphanumeric
  input); each caller decides how to handle that.

### Modified capabilities

- `parse/step` — the step slug derivation relocates to `slug` instead of defining its own
  copy of the rule. No behavior change: `slug` is exactly today's
  `artifact::step::slugify` rule.

- `review` — review filename creation uses `slug`, replacing the punctuation-leaking
  creation path. A title that slugifies to empty is rejected with a typed error.

### Out of scope

- Renaming or migrating any existing on-disk files — the new rule applies only to newly
  created filenames.

- The `NN-` sequential numbering scheme — unchanged.

- `parse_nn_slug`, the read-back parser that splits `NN-<slug>.md` — it reads whatever
  slug is on disk and is not affected.

## Impact

- `plan.rs`'s local `slugify` and `artifact::step::slugify` are deleted; both call the
  shared `slug` rule.

- Punctuated step titles that silently fail `ds check` today begin to pass.

- `duckboard`'s idea filenames become Unicode-aware (non-Latin titles keep their
  characters instead of being folded away), and its `"idea"` empty fallback moves from
  inside `slugify` to the idea creation call site.

- No migration: existing files keep their names; only new files follow the rule.

- New behavior at the creation boundary: an all-non-alphanumeric title now fails fast with
  a typed error instead of producing an unparseable `NN-.md`.
