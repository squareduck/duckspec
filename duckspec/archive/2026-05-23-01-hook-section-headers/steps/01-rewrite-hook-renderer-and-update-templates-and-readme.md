# Rewrite hook renderer and update templates and README

Drop the H1-stripping reader, emit `## Before write` / `## After write` headers in
rendered templates, rename hook files to `<stage>-{before,after}.md`, update all 10 stock
templates, and rewrite the README's hook section.

## Tasks

- [x] 1. Update `read_hook_content` in `crates/duckspec/src/cmd/template.rs`: change the
         path format to `hooks/{stage}-{position}.md` where `position` is `before` /
         `after`; drop H1-skipping entirely; read file, trim leading/trailing whitespace,
         return `None` if empty/whitespace-only.

- [x] 2. Update `apply_hooks` in the same file: match placeholders `## Before write` /
         `## After write`; when the corresponding hook exists emit
         `## Before write\n\n<content>\n\n` (or `## After write`); when absent, drop the
         placeholder line and let `skip_section` consume the trailing blank.

- [x] 3. Update `template::run` callers to pass `"before"` / `"after"` instead of `"pre"`
         / `"post"` when invoking `read_hook_content`.

- [x] 4. Rewrite the two unit tests in `template.rs::tests` to cover four cases: hook
         present (header emitted with content), hook absent (placeholder dropped cleanly),
         empty/whitespace-only hook treated as absent, no-H1 hook content rendered
         verbatim under the header.

- [x] 5. Rename `## Hook - Pre` → `## Before write` and `## Hook - Post` →
         `## After write` across all 10 templates in `crates/duckspec/content/templates/`.

- [x] 6. Add a guard unit test in `template.rs::tests` that walks `TEMPLATE_DIR`, reads
         every `*.md` file, and asserts each contains both `## Before write` and
         `## After write` headers.

- [x] 7. Rewrite the *Customization → Template hooks* section of `README.md` (lines
         189-201) as *Per-stage hooks*: new file naming (`<stage>-before.md` /
         `<stage>-after.md`), explicit-header rendering behavior, no H1 required in the
         hook file, empty file == no hook.
