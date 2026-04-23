# 🦆 duckspec

Spec-driven development framework for AI-assisted coding agents — with a CLI (`ds`) that handles the bookkeeping and a desktop companion (`duckboard`) that puts it all in a window.

Duckspec gives your coding agent a structured workflow for planning, specifying, implementing, and verifying changes. Specs link to tests, docs sit next to specs, and completed work flows through a clear pipeline — so the agent spends tokens on thinking, not chasing broken context.

## What's in this repo

- **`ds`** — the CLI. Initializes duckspec in a project, runs audits, validates artifacts, and emits templates for agents to consume. Install via `cargo install` or the release installer.
- **`duckboard`** — a native macOS GUI for browsing a duckspec project. Project dashboard, change-aware sidebar, cap/codex trees, per-change chat sessions, file finder, project-wide text search, integrated diff view, and a terminal. Install via the release DMG.

Both tools read the same `duckspec/` directory on disk — the CLI is the authoritative engine, and duckboard is a view/editor over the same files.

## Installation

### `ds` CLI

Prebuilt tarballs for macOS (Apple Silicon) and Linux (x86_64) are attached to each [release](https://github.com/squareduck/duckspec/releases/latest):

```sh
# macOS
curl -L https://github.com/squareduck/duckspec/releases/latest/download/ds-<version>-aarch64-apple-darwin.tar.gz \
  | tar -xz && mv ds ~/.local/bin/    # or anywhere on PATH
```

```sh
# Linux
curl -L https://github.com/squareduck/duckspec/releases/latest/download/ds-<version>-x86_64-unknown-linux-gnu.tar.gz \
  | tar -xz && mv ds ~/.local/bin/
```

Or from source (requires [Rust](https://rustup.rs/)):

```sh
cargo install --locked --git https://github.com/squareduck/duckspec.git duckspec
```

### `duckboard` GUI (macOS)

Download `Duckboard-<version>.dmg` from the [latest release](https://github.com/squareduck/duckspec/releases/latest), open it, and drag `Duckboard.app` to `Applications`.

The bundle is currently unsigned — on first launch macOS will warn "unidentified developer". Right-click the app → *Open* → *Open* once to trust it; subsequent launches are normal.

Or from source:

```sh
cargo install --locked --git https://github.com/squareduck/duckspec.git duckboard
```

## Quick Start

```sh
cd your-project
ds init claude   # or: ds init opencode
```

This scaffolds:

- `duckspec/project.md` — high-level project description that agents read for context (fill this in)
- `duckspec/config.toml` — scanning configuration (all fields optional)
- `duckspec/caps/` — capability tree (each capability is a folder with `spec.md` and `doc.md`)
- `duckspec/codex/` — cross-cutting project knowledge outside the change lifecycle
- `duckspec/changes/` — active changes
- `duckspec/archive/` — completed changes
- Agent slash commands with `ds-*` prefix

Run `ds audit` at any time to check project-wide integrity — specs, tests, and docs all stay in sync.

## How It Works

Every piece of work flows through a **change** — an isolated sandbox with its own proposal, design, capability deltas, and execution steps. The agent works through slash commands:

```
/ds-explore → /ds-propose → /ds-design → /ds-spec → /ds-step → /ds-apply → /ds-archive
```

When all steps are complete, `/ds-archive` merges capability deltas into the top-level tree and moves the change into `archive/`.

### Capabilities, specs, docs

Capabilities are the vocabulary duckspec gives your project. Each lives under `duckspec/caps/<path>/` as a folder with two files:

- `spec.md` — formal behavior: requirements, scenarios, invariants. Scenarios tagged `test:code` must be covered by real tests.
- `doc.md` — the same topic in plain prose. Onboarding, rationale, cross-references.

Docs and specs walk in pairs 🦆 — if one exists, both should.

### Specs linked to tests

Source code points back to the scenarios it verifies via `@spec` backlinks in comments. `ds audit` cross-checks that every `test:code` scenario has at least one backlink, and every backlink resolves to a real scenario. Spec drift becomes a build-time error.

### Codex

Not everything belongs in the change lifecycle. `/ds-codex` creates persistent knowledge pages in `duckspec/codex/` — architecture decisions, onboarding guides, cross-cutting rationale. Written directly, no deltas, no archive.

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

## CLI Commands

Commands you'll use directly:

| Command | Description |
|---|---|
| `ds init <harness>` | Initialize a project for an agent harness (`claude`, `opencode`) |
| `ds status [name]` | Show active changes, capability / codex counts, or details for a path |
| `ds audit` | Validate whole project: backlinks, test coverage, cross-artifact integrity |
| `ds check <path>` | Validate specs, steps, codex pages, or entire directories against schemas |
| `ds format <path>` | Rewrite artifacts to canonical markdown in place |
| `ds index` | Print compact project overview (add `--preview` for summaries) |
| `ds sync` | Resolve `@spec` backlinks, update test markers |

Commands the agent calls through slash-command templates: `archive`, `create`, `template`, `schema`.

## Duckboard

`duckboard` is a desktop-grade view over a duckspec project. Open it with ⌘O (or the "Open project" button on the dashboard) and point it at any directory containing a `duckspec/` subfolder — it picks up the project immediately.

What's inside:

- **Dashboard** — active changes, archived changes, in-flight explorations, and a live audit panel that surfaces failing backlinks / missing coverage as you work.
- **Change area** — per-change workspace with an AI chat pane, capability deltas, steps, and a changed-files diff view. Each change remembers its own chat history and tab state.
- **Capability & codex trees** — navigable sidebars over `caps/` and `codex/` with inline spec/doc editing.
- **⌘P file finder** — fuzzy project-wide file search, identical feel to your editor.
- **⌘⇧F text search** — ripgrep-backed project search with scope toggle (whole project vs. `duckspec/` only), file previews, and "stack open every match" mode.
- **Terminal** — per-change PTY tabs for running tests / builds without leaving the window.
- **Recent projects** — the picker remembers where you've been. No project opens by default; pick from the list or browse with tab-completion starting at `~/`.

Duckboard is a companion, not a replacement: it writes to the same files `ds` reads, so you can bounce between CLI and GUI mid-session.

## Configuration

`duckspec/config.toml` controls which files the audit scans for test annotations. All fields are optional — audit works with zero configuration.

```toml
# Directories containing test source files (default: project root)
test_root = "tests"

# Filename glob patterns to select which files to scan
test_patterns = ["*.rs"]

# Path inclusion globs (relative to test root)
test_includes = ["integration/**"]

# Path exclusion globs (relative to test root)
test_excludes = ["fixtures/**"]
```

A file must match `test_patterns`, pass `test_includes`, and not match `test_excludes` to be scanned. Excludes take precedence over includes.

## Customization

### Template hooks

Inject project-specific instructions into any workflow. Place a markdown file at `duckspec/hooks/<stage>-<position>.md`:

```sh
mkdir -p duckspec/hooks
cat > duckspec/hooks/apply-post.md << 'EOF'
Always run `cargo fmt` and `cargo clippy --fix` after modifying Rust files.
EOF
```

Positions are `pre` (runs before the stage's core instructions) and `post` (after).

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
