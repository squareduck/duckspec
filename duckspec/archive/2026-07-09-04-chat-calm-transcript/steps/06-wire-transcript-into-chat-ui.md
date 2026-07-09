# Wire transcript into chat UI

Replace the adjacency `build_chat_blocks` path with segment-backed chat rendering:
Thinking and Activity views, themes, rebuild, and search labels.

## Context

Collapse policy is already in place from step 05:

- `CollapseState { collapsed, user_set }` and `sync_collapse_states` /
  `toggle_collapse` live in `crates/duckboard/src/widget/agent_chat.rs`.
- `AgentSession.chat_collapse` is segment-index-aligned and refreshed from
  `build_transcript_segments` inside `rebuild_chat_editor`.
- The view still reads legacy block-index `chat_collapsed: Vec<bool>` and
  `build_chat_blocks`; task 3 should flip toggles + rendering onto
  `chat_collapse` and drop the one-card-per-tool defaults.

## Prerequisites

- [ ] @step segment-builder-construction-and-pairing
- [ ] @step segment-presentation-helpers
- [ ] @step collapse-policy
- [ ] @step kind-aware-stream-flush

## Tasks

- [x] 1. Extend `BlockKind` (e.g. Reasoning / Activity) and map `TranscriptSeg` into the
         editor block list used by `rebuild_chat_editor`

- [x] 2. Render Thinking (muted, collapsible) and Activity (group card, quiet rows) in
         `agent_chat` view; keep Answer as plain assistant prose

- [x] 3. Drive collapse toggles through the new collapse state; remove one-card-per-tool
         defaults and bare "✓ done" orphan blocks from the builder output

- [x] 4. Update chat search / selection labels for Thinking and Activity kinds in
         `main.rs` / interaction paths

- [x] 5. Remove or retire the old adjacency-only tool merge in `build_chat_blocks` once
         call sites use segments
