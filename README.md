# 🦆 duckspec

Spec-driven development framework for AI coding agents — with a CLI (`ds`) that handles the bookkeeping and a desktop companion (`duckboard`) that puts it all in a window.

Duckspec gives your coding agent a structured workflow for planning, specifying, implementing, and verifying changes. Capabilities are described by paired spec and doc files; source tests link back to the scenarios they verify; and completed work flows through a clear pipeline — so the agent spends tokens on thinking, not chasing broken context.

- **`ds`** — the CLI and the engine. Scaffolds a project, audits integrity, validates artifacts, and emits the slash-command templates agents consume.
- **`duckboard`** — a native macOS app for browsing and driving a duckspec project: dashboard, ideas, per-change AI chat, capability/codex trees, file finder, project search, diff view, and a terminal.

Both tools read and write the same `duckspec/` directory on disk. `ds` is the authoritative engine; duckboard is a view/editor over the same files, and **drives `ds` under the hood** — so `ds` must be installed first (see below).

## Installation

### 1. `ds` CLI (start here)

`ds` is the engine. Install it first — duckboard relies on it, and you'll use it from the terminal too.

**macOS (Apple Silicon):**

```sh
mkdir -p ~/.local/bin && curl -fsSL \
  https://github.com/squareduck/duckspec/releases/latest/download/ds-aarch64-apple-darwin.tar.gz \
  | tar -xz -C ~/.local/bin
```

**Linux (x86_64):**

```sh
mkdir -p ~/.local/bin && curl -fsSL \
  https://github.com/squareduck/duckspec/releases/latest/download/ds-x86_64-unknown-linux-gnu.tar.gz \
  | tar -xz -C ~/.local/bin
```

Make sure `~/.local/bin` is on your `PATH` (most shells already include it). If not:

```sh
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc && exec $SHELL
```

Confirm it works:

```sh
ds --version
```

Or build from source (requires [Rust](https://rustup.rs/)):

```sh
cargo install --locked --git https://github.com/squareduck/duckspec.git duckspec
```

### 2. `duckboard` GUI (optional, macOS)

duckboard is a companion to `ds`, not a replacement — install `ds` first. duckboard runs coding agents that call `ds` on your `PATH`, so the steps above are a prerequisite.

Download `Duckboard-<version>.dmg` from the [latest release](https://github.com/squareduck/duckspec/releases/latest), open it, and drag `Duckboard.app` to `Applications`.

The bundle is ad-hoc signed but not notarized. On first launch macOS blocks it with an "unidentified developer" warning; open *System Settings → Privacy & Security*, scroll to the bottom, and click *Open Anyway* next to Duckboard. Later launches are normal.

Or build from source:

```sh
cargo install --locked --git https://github.com/squareduck/duckspec.git duckboard
```

## Quick start

```sh
cd your-project
ds init claude   # or: ds init opencode
```

`ds init <harness>` creates the `duckspec/` skeleton and installs the agent slash commands:

- `duckspec/caps/` — capability tree (each capability is a folder with `spec.md` + `doc.md`)
- `duckspec/codex/` — cross-cutting project knowledge outside the change lifecycle
- `duckspec/changes/` — active changes
- `duckspec/archive/` — completed changes
- `.claude/commands/ds-*.md` (or `.opencode/commands/`) — the `ds-*` slash commands

Two optional files are written by hand, not by `init`:

- `duckspec/project.md` — a short, high-level description agents read for context.
- `duckspec/config.toml` — scanning / formatting configuration (all fields optional).

Then point your agent at a change and run `/ds-explore`. Run `ds audit` at any time to check project-wide integrity — specs, tests, and docs all stay in sync.

## How it works

Every piece of work flows through a **change** — an isolated sandbox with its own proposal, design, capability deltas, and execution steps. The agent works through slash commands:

```
/ds-explore → /ds-propose → /ds-design → /ds-spec → /ds-step → /ds-apply → /ds-archive
```

When all steps are complete, `/ds-archive` merges the change's capability deltas into the top-level tree and moves the change into `archive/`.

### Capabilities, specs, docs

Capabilities are the vocabulary duckspec gives your project. Each lives under `duckspec/caps/<path>/` as a folder with two files:

- `spec.md` — formal behavior: requirements, scenarios, invariants. Scenarios tagged `test: code` must be covered by real tests.
- `doc.md` — the same topic in plain prose: onboarding, rationale, cross-references.

Docs and specs walk in pairs 🦆 — if one exists, both should.

### Specs linked to tests

Source code points back to the scenarios it verifies via `@spec` backlinks in comments. `ds audit` cross-checks that every `test: code` scenario has at least one backlink and every backlink resolves to a real scenario, so spec drift becomes a build-time error. The same integrity is enforced across the lifecycle: archiving a change won't silently orphan a live backlink (override with `--allow-orphans` when you mean to).

### Codex

Not everything belongs in the change lifecycle. `/ds-codex` creates persistent knowledge pages in `duckspec/codex/` — architecture decisions, onboarding guides, cross-cutting rationale. Written directly: no deltas, no archive.

### Backfill

Adopting duckspec on an existing codebase doesn't need a different workflow — just a different entry point. `/ds-backfill` picks one cohesive slice of existing behavior, uses your tests as the map for what's worth capturing, flags genuine coverage gaps, and hands off to the normal `/ds-propose` → `/ds-spec` flow. Run it again whenever you want to capture another slice; partial coverage is fine indefinitely.

### Workflow paths

Not every change needs every phase. Pick the shape that fits:

**Full feature** — new capabilities and code:

```
/ds-explore → /ds-propose → /ds-design → /ds-spec → /ds-step → /ds-apply → /ds-archive
```

**Doc-only** — updating a capability's doc without changing behavior:

```
/ds-explore → /ds-spec → /ds-archive
```

**Proposal-only** — capturing an idea for later:

```
/ds-explore → /ds-propose → /ds-archive
```

**Spec refinement** — clarifying existing specs without code:

```
/ds-explore → /ds-spec → /ds-archive
```

**Knowledge harvest** — capturing learnings into the codex (no change wrapping):

```
/ds-explore → /ds-codex
```

**Backfill** — capturing existing code into capabilities, one slice at a time:

```
/ds-backfill → /ds-propose → /ds-spec → /ds-archive
```

If the slice's tests have real coverage gaps you want to close in the same change, the path extends through `/ds-step` → `/ds-apply` (and `/ds-design` first if test infrastructure needs designing).

## CLI commands

Commands you'll use directly:

| Command | Description |
|---|---|
| `ds init <harness>` | Initialize a project for an agent harness (`claude`, `opencode`) |
| `ds status [name]` | Show active changes, capability / codex counts, or details for a path |
| `ds audit [change]` | Validate the whole project (or one change): backlinks, test coverage, cross-artifact integrity |
| `ds check <path>` | Validate specs, steps, codex pages, or whole directories against schemas |
| `ds format <path>` | Rewrite artifacts to canonical markdown in place (`--dry` to preview) |
| `ds index` | Print the artifact tree with summaries (filter with `--caps`, `--codex`, `--project`) |
| `ds sync` | Resolve `@spec` backlinks and update test markers |

Commands the agent calls through slash-command templates: `archive`, `create`, `template`, `schema`.

## Duckboard

`duckboard` is a desktop-grade view over a duckspec project. Open it with ⌘O (or the "Open project" button on the dashboard) and point it at any directory containing a `duckspec/` subfolder — it picks up the project immediately. Recent projects are remembered; nothing opens by default.

Areas, switchable from the sidebar:

- **Dashboard** — active changes, archived changes, in-flight explorations, and a live audit panel that surfaces failing backlinks / missing coverage as you work.
- **Ideas** — capture future work as lightweight notes and flow them through *Inbox → Exploration → Change → Archive*. Jot an idea with ⌘I, promote a promising one into a full change, and archived ideas stay linked to the change they became.
- **Change** — a per-change workspace with an AI **chat pane** (its own session history, model selector, and image paste), capability deltas, steps, a changed-files diff view, and a Files explorer.
- **Capability & codex trees** — navigable views over `caps/` and `codex/` with inline spec/doc editing and format-on-save.

Tools that work everywhere:

- **⌘P file finder** — fuzzy project-wide file search, identical feel to your editor.
- **⌘⇧F project search** — ripgrep-backed search with a scope toggle (whole project vs. `duckspec/` only), file previews, and "stack open every match" mode.
- **⌘F local find** — incremental find within the active editor or chat session.
- **Terminal** — per-change PTY tabs for running tests / builds without leaving the window.

Destructive actions are guarded by a confirm-on-arm click, and panics are logged to `~/.config/duckboard/logs/` so issues are easy to report. duckboard writes to the same files `ds` reads, so you can bounce between CLI and GUI mid-session.

## Configuration

`duckspec/config.toml` is optional — duckspec works with zero configuration. It controls which files the audit scans for `@spec` backlinks, and tunes formatting.

```toml
# Directories to scan for @spec backlinks, relative to the project root.
# Empty / omitted means "scan from the project root".
test_paths = ["tests", "src"]

# Files and directories to omit from the backlink scan. Useful for files
# that contain illustrative @spec markers that aren't real backlinks.
exclude = ["references/duckspec.md"]

# Formatting applied by `ds format` and on-save in duckboard.
[format]
line_width = 90   # target wrap width for prose (default: 90)
```

## Customization

### Per-stage hooks

Inject project-specific instructions into any workflow stage. Place a markdown file at `duckspec/hooks/<stage>-<position>.md`, where `<position>` is `before` or `after`:

```sh
mkdir -p duckspec/hooks
cat > duckspec/hooks/apply-after.md << 'EOF'
Always run `cargo fmt` and `cargo clippy --fix` after modifying Rust files.
EOF
```

When you run `ds template <stage>`, the file's contents are inserted into the rendered template under a `## Before write` or `## After write` section header. The body is emitted verbatim — no H1 or other structure required. An empty or whitespace-only file is treated as no hook.

### Schema overrides

Override the embedded writing / conversation guides to match your project's voice:

```sh
ds schema writing-guide > duckspec/hooks/writing-guide.md
# edit the file to taste
```

## Development

This repo is a Cargo workspace:

- `crates/duckpond/` — core library shared by both binaries
- `crates/duckspec/` — CLI crate (binary: `ds`)
- `crates/duckboard/` — GUI crate (binary: `duckboard`)
- `crates/duckchat/` — agent-harness abstraction used by duckboard

Common tasks via [just](https://github.com/casey/just):

```sh
just install       # build and install both binaries to ~/.cargo/bin
just bundle        # build dist/Duckboard.app
just bundle-dmg    # build dist/Duckboard-<version>.dmg
just release 0.2.0 # bump workspace version, commit, tag, push → triggers CI release
```

Version control uses [jujutsu](https://github.com/martinvonz/jj) with a git backend; see `AGENTS.md` for conventions.

## License

MIT
