# duckboard

duckboard is a GUI for the duckspec SDD workflow, built with Iced.
Users drive changes through an embedded terminal (hosting an external
LLM harness like Claude Code), and duckboard provides live visibility
into every artifact, diff, and workflow state. The app watches the
filesystem so changes made by the LLM or editor appear instantly.

## Areas and navigation

The app has four top-level areas, switched via a narrow icon sidebar
on the left (VSCode-style). Only one area is active at a time. Each
area remembers its state when the user switches away and back.

- **Dashboard** --- project overview and navigation hub.
- **Change** --- single change workspace with full editing and
  interaction support.
- **Caps** --- browse the capability tree.
- **Codex** --- browse codex entries.

## Dashboard

Single-panel layout (no three-column split). Contains:

- **Active changes** --- cards or list items showing change name,
  summary (from proposal if present), and step progress. Virtual
  changes appear here with distinct styling (e.g. dashed border).
- **Quick stats** --- capability count, codex entry count, last audit
  status.
- **Archived changes** --- scrollable list below active changes,
  searchable by name.
- **[+ New Change]** button.

Clicking an active or archived change navigates to the Change area
scoped to that change. Clicking a cap or codex entry navigates to the
corresponding area.

## Three-column layout

The Change, Caps, and Codex areas share a three-column layout:

```
┌──────────────┬──────────────────────┬───────────────────────┐
│  List        │  Content             │  Interaction          │
│              │                      │                       │
│  collapsible │  tabbed viewer       │  terminal + chat tabs │
│  sections,   │  pin support         │  collapsible to right │
│  tree views, │  structured/raw/edit │  lazy-spawned         │
│  search      │                      │                       │
└──────────────┴──────────────────────┴───────────────────────┘
```

### List column

Shows collapsible sections per artifact type. Each section supports
tree views where applicable. A search/filter input at the top narrows
the visible tree by fuzzy matching on names and paths.

**Change area sections:**

1. **Overview** --- proposal.md, design.md (flat list).
2. **Capabilities** --- tree of cap paths within the change, each
   expanding to show spec.md, spec.delta.md, doc.md, doc.delta.md as
   applicable.
3. **Steps** --- flat numbered list. Each item shows checked/unchecked
   task progress.
4. **Files** _(future)_ --- repository files changed by this change,
   sourced from VCS. See "Future: VCS integration" below.

A change selector dropdown at the top of the list column allows
switching between active changes without returning to the dashboard.

**Caps area sections:**

- **Capabilities** --- tree matching `duckspec/caps/` folder
  structure. Each leaf capability expands to show spec.md and doc.md.
  Audit coverage information is overlaid per capability (colored dot
  or badge indicating whether scenarios have test backlinks).

**Codex area sections:**

- **Entries** --- flat or shallow tree matching `duckspec/codex/`
  structure.

### Content column

Tabbed viewer for selected items from the list column. Selecting an
item in the list opens it as a tab. Tabs can be pinned.

**Tab management:** a configurable cap on maximum open tabs (e.g. 10).
When the cap is reached, the oldest unpinned tab is evicted (LRU).
Pinned tabs are never evicted. Pinned tabs survive area switches.

**Rendering modes** (toggled per tab):

- **Structured view** (default for duckspec artifacts) --- parsed
  using duckpond into semantic UI. Requirements rendered as blocks,
  scenarios with formatted GWT clauses, test markers as badges, step
  tasks as checklists.
- **Raw view** --- file contents with syntax highlighting.
- **Edit mode** --- activated from raw view via an edit button.
  Basic text editor. Saving writes to disk; the file watcher picks up
  the change and the structured view updates. Edits are validated
  through duckpond (`check_artifact`) with inline error reporting.

**Delta rendering** has three sub-views:

- The delta itself (structured).
- Merge preview (result of applying delta to base cap).
- Diff view (base vs. merged, with highlighted additions/removals).

**Future source file rendering** (see "VCS integration"):

- Syntax-highlighted source viewer.
- Diff toggle showing VCS changes.

The content renderer should be designed as an extensible enum/trait
from the start to support adding new file types:

- `.md` recognized as duckspec artifact --- structured + raw + edit.
- `.md` not recognized --- markdown render + raw + edit.
- Source files (`.rs`, `.ts`, etc.) --- syntax highlighted + diff.
- Unknown --- raw text.

### Interaction column

Collapsible to the right edge. Present in every area, not just
Change. Before first use, shows a prompt to create a terminal or chat
session rather than auto-spawning processes.

**Terminal tabs:** multiple terminal tabs per scope. The user can
create additional terminals as needed. Terminals persist for the app
lifetime within their scope.

**Chat tab:** stub for now. Will later host a custom LLM harness
built into duckboard. For now, users run their LLM harness (Claude
Code, etc.) in the terminal.

**Terminal scoping:**

- Each change (active, virtual, or archived) gets its own terminal
  pool. Archived changes have full terminals (useful for asking the
  LLM questions about the change).
- The Caps area gets its own terminal pool.
- The Codex area gets its own terminal pool.
- The Dashboard does not have an interaction column.

## Virtual change lifecycle

A virtual change allows users to start exploring before committing to
a change structure on disk. Only one virtual change may exist at a
time.

```
[+ New Change]  --->  Virtual Change
                      - no duckspec/changes/<name>/ folder on disk
                      - terminal available (run /ds-explore, talk to LLM)
                      - list column: empty or minimal
                      - content column: welcome/guide view

[file watcher detects new changes/<name>/]  --->  Materialized Change
                      - terminal session continues seamlessly
                      - list column: populates with detected artifacts
                      - content column: artifacts become openable

[ds archive applied]  --->  Archived Change
                      - moved to archive/ on disk
                      - terminal still available (for questions/review)
                      - list column: frozen artifacts
                      - content column: read-only structured/raw views
```

When a new folder appears under `duckspec/changes/` while a virtual
change exists, duckboard automatically claims it and transitions the
virtual change to a materialized change.

## File watching and reactivity

duckboard watches `duckspec/` recursively. On any filesystem change:

- Re-parse affected artifacts using duckpond.
- Update the list column tree in the relevant area.
- If a currently-viewed artifact changed, refresh the content tab.
- Briefly highlight changed items in the list for visibility.

This ensures that changes made by the LLM in the terminal, by an
external editor, or by `ds` CLI commands are reflected instantly.

## Validation and audit overlay

Validation (via duckpond `check_artifact`) runs:

- On file save in edit mode, with inline error display.
- On file watcher events, updating per-artifact status indicators.

Audit information (from duckpond's backlink/coverage analysis) is
overlaid in the Caps area list column:

- Per-capability coverage indicator (e.g. green/yellow/red dot).
- Green: all `test:code` scenarios have backlinks.
- Yellow: partial coverage.
- Red: no coverage or unresolved backlinks.

## Future: VCS integration

A future addition to the Change area list column: a **Files** section
showing all repository files changed by the change, powered by
git/Jujutsu integration.

This requires:

- VCS integration layer (git and Jujutsu support) to determine which
  files changed.
- Syntax-highlighted source file viewer in the content column.
- Code diff rendering (actual VCS diffs, not duckspec deltas).
- Linking `@spec` backlinks found in changed source files back to the
  corresponding capabilities in the cap tree.

This closes the full workflow loop: the interaction column drives
changes via the LLM, the duckspec sections show the spec artifacts,
and the Files section shows the resulting code changes. Full cycle
without leaving the app.

## State model

```
AppState
  active_area: Dashboard | Change(name) | Caps | Codex
  changes: Map<name, ChangeState>
    artifacts: parsed artifact tree (kept current by file watcher)
    terminals: Vec<TerminalSession>
    chat: Option<ChatSession>        (stub for now)
    is_virtual: bool
    is_archived: bool
  virtual_change: Option<VirtualChangeState>
    terminals: Vec<TerminalSession>
    chat: Option<ChatSession>
  caps: CapTree                      (parsed from duckspec/caps/)
  caps_terminals: Vec<TerminalSession>
  codex: CodexTree                   (parsed from duckspec/codex/)
  codex_terminals: Vec<TerminalSession>
  archive: Vec<ArchivedChange>
  file_watcher: WatcherHandle
  tab_state: Map<AreaKey, TabState>
    open_tabs: Vec<Tab>              (capped, LRU eviction)
    pinned_tabs: Set<TabId>
```
