# Update critique handoff stock content

Replace review/followup Handoff sections and drop `ignore` from style next-token examples.

## Tasks

- [x] 1. Replace `## Handoff` in `crates/duckspec/content/templates/review.md` with the
         design wording: `investigate` when murky; `/ds-spec` before `/ds-step` for
         behavior/invariants; omit the card when nothing useful; no `ignore` and no
         always-emit

- [x] 2. Apply the same Handoff replacement in
         `crates/duckspec/content/templates/followup.md` (identical text to review)

- [x] 3. Remove `` `ignore` `` from the `next` meta card send-token example list in
         `crates/duckspec/content/schemas/style.md`

- [x] 4. Smoke-check: `ds template review`, `ds template followup`, and `ds schema style`
         match design; no `` `ignore` `` in handoff or token lists;
         `cargo test -p duckspec --test stock_content` green
