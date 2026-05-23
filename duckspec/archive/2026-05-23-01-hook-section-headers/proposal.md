# Per-stage hooks with explicit section headers

Replace the H1-stripping hook injection in `ds template <stage>` with explicit
`## Before write` / `## After write` headers in the rendered output, and rename hook files
from `<stage>-pre.md` / `<stage>-post.md` to `<stage>-before.md` / `<stage>-after.md` so
file naming and rendered headers share one vocabulary.

## Motivation

The current hook injection in `crates/duckspec/src/cmd/template.rs` has two flaws that hit
on first use.

First, it requires the hook file to start with an H1 line. The reader strips everything up
to and including the first `# ...`, then inserts the remainder. Any file without an H1 is
silently rendered as empty — the section the hook was meant to fill just disappears. There
is no warning, and the failure looks identical to "no hook file exists."

Second, hook content is inserted as raw prose between the template's H2 sections. In the
rendered output the project's customization is visually indistinguishable from duckspec's
stock template prose. The agent reading the rendered template cannot tell "this paragraph
is your project's standing instruction" from "this paragraph is duckspec's generic
guidance."

Both flaws disappear once the rendered output carries an explicit section header above the
hook content and the reader treats the file body as-is.

## Scope

Pure CLI refactor — no capability changes. Touched surfaces:

- `crates/duckspec/src/cmd/template.rs` — drop H1-skipping in `read_hook_content`; rewrite
  `apply_hooks` to emit `## Before write` / `## After write` header + body when a hook is
  present, and remove the placeholder entirely when absent; update unit tests.

- `crates/duckspec/content/templates/*.md` (10 files) — rename `## Hook - Pre` →
  `## Before write` and `## Hook - Post` → `## After write`.

- `README.md` — rewrite the *Customization → Template hooks* section as *Per-stage hooks*
  with the new file naming and rendering rules.

### Out of scope

- Hooks outside per-stage templates (e.g. per-command or per-artifact hooks).

- Multiple hooks per stage or cross-stage hook composition.

- The `ds schema writing-guide > duckspec/hooks/writing-guide.md` flow under README's
  *Schema overrides* — different mechanism, separate concern.

## Impact

- **Breaking for any existing hook files.** Users with `duckspec/hooks/<stage>-pre.md` or
  `<stage>-post.md` must rename to `-before.md` / `-after.md`. No in-repo files use the
  old names today.

- **Templates ship with renamed placeholders.** Anyone with a fork of the stock templates
  needs to update the H2 placeholder names if they want hooks to resolve.

- **README customization section is renamed.** External links to the *Template hooks*
  anchor will break; the new anchor is *Per-stage hooks*.
