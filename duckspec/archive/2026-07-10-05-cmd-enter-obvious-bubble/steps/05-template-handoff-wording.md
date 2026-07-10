# Template handoff wording

Reword workflow template Handoff sections from bold Primary/Secondary labels to a flat
“Suggested next actions:” bullet list. Rank order and the ≤2 rule stay the same.

## Prerequisites

- [x] @step bubble-chrome-cmd-enter-and-send

## Tasks

- [x] 1. Update Handoff sections under `crates/duckspec/content/templates/` (`explore`,
         `propose`, `design`, `spec`, `step`, `apply`, `review`, `archive`, `backfill`,
         `codex`, and any other templates that emit `**Primary**` / `**Secondary**`) to
         the flat list shape:

- [x] 2. Rewrite instruction prose that tells agents to offer “(**Primary**, then
         **Secondary**…)” so it describes ranked list order instead (explore template and
         similar)

- [x] 3. Keep stage matrices and policy (e.g. apply handoff still ranks review before
         archive in agent prose) — only the presentation labels change

- [x] 4. Spot-check that no main-flow handoff gains a third ranked action or reintroduces
         `/ds-verify` as a ranked next stage
