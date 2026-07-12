# Settings group heading hierarchy

Make Settings group headings (`Global` / `This project`) read above field titles via
`font_lg` and extra space between those peer sections.

## Context

Followup `reviews/01-followup-settings-group-heading-hierarchy.md`: groups currently use
`font_md`, same as field labels. Theme documents a third size tier but only exposes
`font_sm` / `font_md`.

## Tasks

- [x] 1. Add `font_lg()` in `crates/duckboard/src/theme.rs` as `ui_size() + 2` (documented
         third tier)

- [x] 2. Use `theme::font_lg()` for `Global` and `This project` headings in
         `crates/duckboard/src/area/settings.rs`; leave field titles at `font_md`

- [x] 3. Increase vertical space between the Global block and the This project heading
         (more than a single `SPACING_XL`, or an extra gap after Global)
