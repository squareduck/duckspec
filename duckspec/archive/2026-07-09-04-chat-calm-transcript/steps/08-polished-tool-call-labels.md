# Polished tool call labels

Make Activity tool rows and collapsed summaries look calm and intentional — humanized
labels for common Claude and Grok tools, and a graceful fallback for everything else so
unknown tools never dump raw harness noise.

## Prerequisites

- [ ] @step segment-presentation-helpers
- [ ] @step wire-transcript-into-chat-ui

## Context

Presentation polish on top of the calm transcript segment model. No new cap scenarios
required unless you discover behavior that should be locked as a requirement — freeform
unit tests are enough for this step.

**Problem today** (`format_tool_summary` / `tool_display_name` in
`crates/duckboard/src/widget/agent_chat.rs`):

- Grok snake_case titles (`read_file`, `run_terminal_command`, `search_replace`) and
  Claude names (`Bash`, `Read`, `Edit`) share rows but read inconsistently.

- Detail extraction only knows `path`/`file_path`, `pattern`, and `command` — misses
  common shapes (e.g. write `contents`, replace `old_string`/`new_string`, search
  `query`).

- Unknown tools fall back to the bare raw name (or name + nothing) — janky when input is a
  big JSON object.

**Goal:** one quiet line per tool that always looks designed.

```
✓ Read · agent_chat.rs
● Shell · cargo test -p duckboard
✓ Grep · "format_tool_summary"
✓ Edit · state.rs
✓ some_obscure_tool · primary-arg-or-short-hint
```

Collapsed Activity samples use the same short human names (`3 tools · Read, Shell, Grep`),
not harness ids.

**Where:** primarily `format_tool_summary` and `tool_display_name` in
`crates/duckboard/src/widget/agent_chat.rs` (and any small helpers they grow). Rows still
flow through `activity_body_lines` / `activity_collapsed_label` — no view-layer fork per
harness.

**Design notes for the applying agent:**

- Map known aliases to a single short display verb (case-insensitive), e.g.:

  - shell: `Bash`, `run_terminal_command`, `shell` → **Shell**

  - read: `Read`, `read_file` → **Read**

  - write: `Write`, `write` → **Write**

  - edit: `Edit`, `search_replace`, `MultiEdit` → **Edit** (or **Replace** if you prefer
    one label for replace-shaped tools — pick one and use it consistently)

  - search: `Grep`, `grep` → **Grep**

  - list: `LS`, `list_dir`, `Glob`, `glob` → **List** / **Glob** as appropriate

  - web: `WebSearch`, `web_search`, `WebFetch`, `open_page` → short calm labels

  - Keep the table small and maintainable; don't invent a framework.

- Detail: prefer path (shortened), command (truncated, single line), pattern/query
  (quoted, truncated). Never paste multi-line bodies or full JSON into the summary line.

- **Unknown tools must still look good:** humanize the name (`snake_case` / `camelCase` →
  readable words, or Title Case short form), then attach at most one short detail — first
  useful string field, or a single truncated key hint. If there's nothing useful, name
  alone is fine. No raw `{...}` blobs.

- Use a calm separator (e.g. middle-dot `·`) between verb and detail for consistency with
  Thinking/Activity labels elsewhere.

- Multibyte-safe truncation stays required (existing `truncate_chars` tests).

## Tasks

- [x] 1. Introduce a small pure helper that maps known Claude/Grok tool names to a short
         human display verb (case-insensitive aliases); leave unmapped names for the
         humanize fallback

- [x] 2. Expand detail extraction from tool input JSON: path/file_path, command,
         pattern/query, and other common single-line fields — never multi-line bodies or
         full JSON dumps

- [x] 3. Rewrite `format_tool_summary` to compose `Verb · detail` (or verb alone) for
         known tools and the same calm shape for unknown tools after name humanization

- [x] 4. Ensure collapsed Activity sample names (`tool_display_name` /
         `activity_collapsed_label`) use the humanized verb, not raw harness ids like
         `run_terminal_command`

- [x] 5. Unit tests: Claude-style names (`Read`, `Bash`, `Grep`, `Edit`) and Grok-style
         names (`read_file`, `run_terminal_command`, `search_replace`) produce calm,
         consistent labels with the right detail

- [x] 6. Unit tests: unknown tool names look intentional (humanized name, optional short
         detail, no raw JSON); empty/minimal input still renders cleanly
