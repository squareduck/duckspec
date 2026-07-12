# Spec template map plus first outline

Reshape the stock spec template so the map turn ends with the first capability's outline
gate and every later `confirm` lands on a tool-anchored write turn.

## Tasks

- [x] 1. In `crates/duckspec/content/templates/spec.md`, merge Instructions steps 2-3: the
         map turn presents the map and then the first capability's outline write gate;
         `confirm` approves both. Each post-confirm turn creates/writes/formats/checks the
         confirmed capability, then presents the next capability's outline gate in the
         same turn; Handoff after the last write

- [x] 2. Align the Write gate section: replace the map-only gate shape with map +
         first-outline (single trailing `next` meta card with `confirm`); keep
         CREATE/UPDATE outline shapes unchanged

- [x] 3. Reinstall `ds` (`cargo install --path crates/duckspec`) and verify
         `ds template spec` renders the new flow

- [x] 4. Add `# REMOVE <path>` to the map vocabulary with a REMOVE gate shape (no outline;
         delta files with a `-` H1 on write) for genuinely retired capabilities
