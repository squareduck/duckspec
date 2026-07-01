# Unify slug generation — Design

A new `duckpond::slug` module owns one pure title-to-slug rule; the three existing copies
collapse onto it, and each caller decides its own empty-slug policy.

## Approach

```
                    ┌───────────────────────────────┐
                    │  duckpond::slug               │
                    │                               │
                    │  pub fn slugify(&str) -> String
                    │  (map non-alphanumeric → '-', │
                    │   Unicode-aware, collapse,    │
                    │   trim; may return "")        │
                    └───────────────┬───────────────┘
                                    │ single source of truth
          ┌─────────────────────────┼─────────────────────────┐
          │                         │                         │
          ▼                         ▼                         ▼
  parse/step.rs             plan.rs (create)          duckboard
  derive slug from H1       create_step / create_review  idea_store.rs
  (compare vs filename)     reject "" → EmptySlug      fall back to "idea"
```

The rule already exists — verbatim — as `artifact::step::slugify`. This change does not
invent a rule; it *relocates* that rule to a module both crates can share and repoints
every caller at it. The two extra copies (`plan.rs`, `duckboard::idea_store`) are deleted.

Empty-slug handling is deliberately **not** baked into the shared function. `slugify` is a
pure transformation that may legitimately return `""` (an all-punctuation title). Each
caller owns the policy: `ds create` treats it as a user error and rejects; duckboard
substitutes its `"idea"` placeholder. Keeping policy at the boundary is what lets one
function serve both.

## `duckpond::slug` module

New top-level module `crates/duckpond/src/slug.rs`, declared `pub mod slug;` in `lib.rs`.
It holds nothing but the rule and its unit tests — the smallest possible home for the
single source of truth.

```rust
/// Convert a human title into a kebab-case slug.
///
/// Lowercases, keeps Unicode alphanumerics, maps every run of other characters
/// to a single `-`, and trims leading/trailing `-`. Returns an empty string
/// when the input has no alphanumeric characters; callers decide how to treat
/// that.
pub fn slugify(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
```

This body is lifted unchanged from `artifact::step::slugify`, so step slug derivation
keeps its exact current behavior.

## `parse/step` derivation

`parse/step.rs:31` currently calls the local `slugify(&title)` to derive a step's slug
from its H1 title, which `check.rs` compares against the filename. Repoint it at the
shared rule and delete the definition in `artifact/step.rs`.

```rust
// parse/step.rs
let slug = crate::slug::slugify(&title);
```

Pure relocation: the shared rule is byte-for-byte the old one, so no step slug changes and
no existing `ds check` result moves. The `pub fn slugify` at `artifact/step.rs:66` and its
unit tests move to `slug.rs`.

## `plan.rs` creation

`create_step` (`plan.rs:243`) and `create_review` (`plan.rs:304`) call the local
`slugify`, which is the buggy one. Repoint both at `crate::slug::slugify`, delete the
local copy (`plan.rs:393`), and reject an empty slug before building the path — an empty
slug would produce an unparseable `NN-.md` and corrupt numbering.

```rust
#[derive(Debug, thiserror::Error)]
pub enum PlanError {
    // ...existing variants...
    #[error("cannot derive a slug from title '{title}': no alphanumeric characters")]
    EmptySlug { title: String },
}

// in create_step and create_review, before uniqueness/numbering:
let slug = crate::slug::slugify(name);
if slug.is_empty() {
    return Err(PlanError::EmptySlug { title: name.to_string() });
}
```

The uniqueness checks (`StepSlugExists`, `ReviewSlugExists`) and numbering are unchanged
and still run on the now-clean slug.

## `duckboard::idea_store`

Delete `idea_store::slugify` (`:163`) and call `duckpond::slug::slugify` at the two sites
that use it. The `"idea"` empty fallback moves out of the function and onto the title
site; the tag-segment site keeps its existing "skip empty segments" behavior.

```rust
// title (idea_store.rs:425)
let raw = duckpond::slug::slugify(&idea.frontmatter.title);
let slug = if raw.is_empty() { "idea".to_string() } else { raw };

// tag segments (primary_tag_segments, idea_store.rs:186)
primary
    .split('/')
    .map(str::trim)
    .filter(|s| !s.is_empty())
    .map(duckpond::slug::slugify)
    .filter(|s| !s.is_empty())   // drop segments that slugify to empty
    .collect()
```

Idea filenames become Unicode-aware as a result: a non-Latin title that folded to `"idea"`
under the old ASCII rule now keeps its characters. Ideas live under the app data
directory, not the git-tracked `duckspec/` tree, so Unicode filenames carry no
cross-repository portability concern.

## Decisions

- **The rule is the existing step rule — one bucket, not three.** Every non-alphanumeric
  maps to `-`; nothing is dropped. Alternatives considered and rejected: *dropping*
  punctuation while keeping some as separators (inconsistent — `foo:bar` vs `foo-bar` vs
  `foo_bar` all diverge) and a multi-bucket separator set (needless complexity for no real
  gain). One uniform mapping is simplest and already proven in `artifact::step::slugify`.

- **`slugify` returns `String`; empty-slug policy lives at each caller.** Alternative:
  return `Option<String>` or bake in a fallback. Rejected: step derivation wants the raw
  string to compare against the filename, `ds create` wants a hard error, and duckboard
  wants an `"idea"` placeholder — three different policies, so the shared function must
  stay policy-free.

- **New top-level `duckpond::slug` module.** Alternative: leave the rule in
  `artifact::step` and have everyone call it. Rejected: `artifact::step` is about parsing
  a step, not general naming, and duckboard importing a step-parser helper for idea
  filenames is a confusing dependency. A dedicated module matches the house style
  (`parse/elements`, `merge/validate`).

- **Unicode alphanumerics over ASCII fold.** Alternative: ASCII-only (idea's current
  behavior). Rejected: idea titles are frequently non-English, and ASCII folding silently
  discards their content; ideas are not git-tracked, so the portability argument for ASCII
  does not apply.

## Risks

- **duckboard idea filenames shift from ASCII to Unicode.** → New files only; existing
  idea files are never renamed. The behavior is strictly less lossy, and ideas live
  outside the versioned tree.

- **The shared `slugify` can now return `""`, which the old per-call copies masked.** →
  Every caller handles it explicitly: `ds create` rejects with `EmptySlug`, duckboard
  falls back to `"idea"`, tag segments filter empties. Covered by unit tests at each site.
