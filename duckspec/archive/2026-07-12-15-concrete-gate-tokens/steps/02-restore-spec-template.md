# Restore spec template

Rewrite the stock spec template to the original separate map stage, with REMOVE vocabulary
and path-qualified confirm tokens.

## Prerequisites

- [x] @step style-gate-token-rules

## Tasks

- [x] 1. Restore separate map → `confirm map` → per-cap outline+write flow in
         `crates/duckspec/content/templates/spec.md` (not map+first, not split
         outline/write)

- [x] 2. Keep `# REMOVE <path>` on the map and a REMOVE capability gate shape

- [x] 3. Wire concrete tokens: `confirm map`, `confirm <path>`, `confirm remove <path>`
         with no reason text on those lines

- [x] 4. Align Voice, Instructions, Chat, and Write gate prose with the restored flow and
         concrete tokens (no bare `confirm` wait instructions)
