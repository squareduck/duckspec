# 🦆 duckspec

Spec-driven development framework for AI coding agents — with a CLI (`ds`) that handles the bookkeeping and a desktop companion (`duckboard`) that puts it all in a window.

Duckspec gives your coding agent a structured workflow for exploring, proposing, designing, specifying, implementing, reviewing, and archiving changes. Capabilities are described by paired spec and doc files; source tests link back to the scenarios they verify; and completed work flows through a clear pipeline — so the agent spends tokens on thinking, not chasing broken context.

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

The app bundle includes harness ACP agents (`duckchat-claude-acp`, `duckchat-codex-acp`)
next to the `duckboard` binary. Finder-launched Duckboard does **not** need those agents
on your `PATH` for Claude or Codex turns.

The bundle is ad-hoc signed but not notarized. On first launch macOS blocks it with an "unidentified developer" warning; open *System Settings → Privacy & Security*, scroll to the bottom, and click *Open Anyway* next to Duckboard. Later launches are normal.

Or build from source (install the GUI and harness agents — cargo install does not bundle
them together the way the DMG does):

```sh
cargo install --locked --git https://github.com/squareduck/duckspec.git duckboard
cargo install --locked --git https://github.com/squareduck/duckspec.git duckchat-claude-acp
cargo install --locked --git https://github.com/squareduck/duckspec.git duckchat-codex-acp
```

That puts them on `~/.cargo/bin` (must be on your `PATH` when you launch duckboard from a
terminal). From a clone you can also use `just install` to install `ds`, `duckboard`, and
both harness agents together.

### 3. Agent harnesses (upstream CLIs)

Duckboard turns run on a **harness** — a backend agent. Each harness needs its **upstream
CLI** installed and authenticated separately. Auth is always the upstream product’s
(Claude / ChatGPT / SuperGrok / etc.); duckboard does not replace that.

| Harness | duckboard id | `ds init` | CLI on `PATH` | Install / docs |
| --- | --- | --- | --- | --- |
| Claude Code | `claude-code` | `claude` | `claude` | [Claude Code quickstart](https://code.claude.com/docs/en/quickstart) |
| Grok Build | `grok` | — | `grok` | [Grok Build CLI](https://x.ai/cli) · [docs](https://docs.x.ai/build/overview) |
| OpenAI Codex | `openai-codex` | `codex` | `codex` | [Codex CLI](https://developers.openai.com/codex/cli) |
| OpenCode | — (skills only) | `opencode` | `opencode` | [OpenCode install](https://opencode.ai/docs/) |

**What duckboard ships vs what you install:**

- **Owned ACP agents** (`duckchat-claude-acp`, `duckchat-codex-acp`) ship next to duckboard
  in the DMG / `just bundle`, or via `cargo install` / `just install`. Override discovery
  with `DUCKCHAT_CLAUDE_ACP` / `DUCKCHAT_CODEX_ACP` (then sibling of the running exe, then
  `PATH`). See [Development](#development).
- **Upstream CLIs** (`claude`, `grok`, `codex`) are **not** bundled. Install them from the
  links above, sign in as that product requires, and keep them on your `PATH`.
- Grok does not use a separate owned ACP binary; duckboard launches the official `grok`
  CLI directly.
- `ds init opencode` only installs stage skills under `.opencode/commands/`; it is not a
  duckboard model-catalog harness.

Quick install examples (see each product’s docs for updates and Windows):

```sh
# Claude Code
curl -fsSL https://claude.ai/install.sh | bash

# Grok Build
curl -fsSL https://x.ai/cli/install.sh | bash

# Codex CLI
curl -fsSL https://chatgpt.com/codex/install.sh | sh

# OpenCode (skills / terminal agent)
curl -fsSL https://opencode.ai/install | bash
```

## Quick start

Pick a harness whose upstream CLI you already installed (section 3), then:

```sh
cd your-project
ds init claude   # or: ds init opencode | ds init codex
```

`ds init <harness>` creates the `duckspec/` skeleton and installs stage commands/skills for
that harness. You still need the matching upstream CLI on `PATH` for live agent turns
(except that OpenCode is install-for-skills only relative to duckboard).

- `duckspec/caps/` — capability tree (each capability is a folder with `spec.md` + `doc.md`)
- `duckspec/codex/` — cross-cutting project knowledge outside the change lifecycle
- `duckspec/changes/` — active changes
- `duckspec/archive/` — completed changes
- `.claude/commands/ds-*.md`, `.opencode/commands/`, or `.agents/skills/*/SKILL.md` (codex) —
  the `ds-*` stage commands/skills

Two optional files are written by hand, not by `init`:

- `duckspec/project.md` — a short, high-level description agents read for context.
- `duckspec/config.toml` — scanning / formatting configuration (all fields optional).

Then point your agent at a change and run `/ds-explore`. Run `ds audit` at any time to check project-wide integrity — specs, tests, and docs all stay in sync.

## How it works

Every piece of work flows through a **change** — an isolated sandbox with its own proposal, design, capability deltas, execution steps, and critique history. The agent works through slash commands. The usual spine:

```
/ds-explore → /ds-propose → /ds-design → /ds-spec → /ds-step → /ds-apply
    → /ds-review  (and/or /ds-followup)
    → /ds-archive
```

When all steps are complete (and any rework is done), `/ds-archive` merges the change's capability deltas into the top-level tree and moves the change into `archive/`. Review and followup can also run while steps are still open.

Side stages sit outside that spine: `/ds-verify` (validate without writing), `/ds-codex` (project knowledge), and `/ds-backfill` (capture existing code).

### Capabilities, specs, docs

Capabilities are the vocabulary duckspec gives your project. Each lives under `duckspec/caps/<path>/` as a folder with two files:

- `spec.md` — formal behavior: requirements, scenarios, invariants. Scenarios tagged `test: code` must be covered by real tests.
- `doc.md` — the same topic in plain prose: onboarding, rationale, cross-references.

Docs and specs walk in pairs 🦆 — if one exists, both should.

### Specs linked to tests

Source code points back to the scenarios it verifies via `@spec` backlinks in comments. `ds audit` cross-checks that every `test: code` scenario has at least one backlink and every backlink resolves to a real scenario, so spec drift becomes a build-time error. The same integrity is enforced across the lifecycle: archiving a change won't silently orphan a live backlink (override with `--allow-orphans` when you mean to).

### Review and followup

`ds check` and `ds audit` prove a change is well-*formed*. Critique records whether it is well-*conceived* and well-*made*. Both kinds append to the same log under `duckspec/changes/<name>/reviews/`:

| Skill | Who drives it | File shape |
| --- | --- | --- |
| `/ds-review` | Agent-led scan of the change chain | `NN-review-<slug>.md` |
| `/ds-followup` | User-led course correction in conversation | `NN-followup-<slug>.md` |

Each pass is a new file (append-only), not an edit of the previous one. Records stay advisory: they recommend next stages (`/ds-spec`, `/ds-step`, `/ds-apply`, …) but do not implement plan or product code unless you ask after the document exists.

### Verify

`/ds-verify` is a diagnostic side path — run `ds check`, `ds audit`, and a dry `ds sync`, report what is clean and what is not, then stop. It does not create or edit artifacts. Use it anytime you want a health check without entering a lifecycle stage.

### Codex

Not everything belongs in the change lifecycle. `/ds-codex` creates persistent knowledge pages in `duckspec/codex/` — architecture decisions, onboarding guides, cross-cutting rationale. Written directly: no deltas, no archive.

### Backfill

Adopting duckspec on an existing codebase doesn't need a different workflow — just a different entry point. `/ds-backfill` picks one cohesive slice of existing behavior, uses your tests as the map for what's worth capturing, flags genuine coverage gaps, and hands off to the normal `/ds-propose` → `/ds-spec` flow. Run it again whenever you want to capture another slice; partial coverage is fine indefinitely.

### Workflow paths

Not every change needs every phase. Pick the shape that fits:

**Full feature** — new capabilities and code:

```
/ds-explore → /ds-propose → /ds-design → /ds-spec → /ds-step → /ds-apply
    → /ds-review  (and/or /ds-followup)
    → /ds-archive
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

**Critique / rework** — judgment or course correction mid-change or before archive:

```
/ds-review     → /ds-spec or /ds-step or /ds-apply → …
/ds-followup   → /ds-spec or /ds-step or /ds-apply → …
```

**Knowledge harvest** — capturing learnings into the codex (no change wrapping):

```
/ds-explore → /ds-codex
```

**Health check** — validate without writing:

```
/ds-verify
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
| `ds init <harness>` | Initialize a project for an agent harness (`claude`, `opencode`, `codex`) |
| `ds status [name]` | Show active changes, capability / codex counts, or details for a path |
| `ds audit [change]` | Validate the whole project (or one change): backlinks, test coverage, cross-artifact integrity |
| `ds check <path>` | Validate specs, steps, codex pages, or whole directories against schemas |
| `ds format <path>` | Rewrite artifacts to canonical markdown in place (`--dry` to preview) |
| `ds index` | Print the artifact tree with summaries (filter with `--caps`, `--codex`, `--project`) |
| `ds sync` | Resolve `@spec` backlinks and update test markers |

Commands the agent calls through slash-command templates: `archive`, `create`, `template`, `schema`. Agent stages installed by `ds init` include `explore`, `propose`, `design`, `spec`, `step`, `apply`, `archive`, `review`, `followup`, `verify`, `codex`, and `backfill`.

## Duckboard

`duckboard` is a desktop-grade view over a duckspec project. Open it with ⌘O (or the "Open project" button on the dashboard) and point it at any directory containing a `duckspec/` subfolder — it picks up the project immediately. Recent projects are remembered; nothing opens by default.

Areas, switchable from the sidebar:

- **Dashboard** — active changes, archived changes, in-flight explorations, and a live audit panel that surfaces failing backlinks / missing coverage as you work.
- **Ideas** — capture future work as lightweight notes and flow them through *Inbox → Exploration → Change → Archive*. Jot an idea with ⌘I, promote a promising one into a full change, and archived ideas stay linked to the change they became.
- **Change** — a per-change workspace with an AI **chat pane** (its own session history, multi-harness model picker including Claude and Grok, and image paste), capability deltas, steps, a changed-files diff view, and a Files explorer.
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

Inject project-specific instructions into any workflow stage. Place a markdown file at `duckspec/hooks/<stage>-<position>.md`, where `<position>` is `before` or `after` (for example `apply-after.md`, `spec-before.md`):

```sh
mkdir -p duckspec/hooks
cat > duckspec/hooks/apply-after.md << 'EOF'
Always run `cargo fmt` and `cargo clippy --fix` after modifying Rust files.
EOF
```

When you run `ds template <stage>`, the file's contents are inserted into the rendered template under a `## Before write` or `## After write` section header. The body is emitted verbatim — no H1 or other structure required. An empty or whitespace-only file is treated as no hook. Stage names match the agent templates (`explore`, `propose`, `design`, `spec`, `step`, `apply`, `archive`, `review`, `followup`, `verify`, `codex`, `backfill`).

### Schemas and style

Agents load embedded artifact and style guides with `ds schema <name>` (for example `ds schema style`, `ds schema proposal`, `ds schema review`). These describe how chat and on-disk markdown should look; they are not project-local override files. Prefer per-stage hooks when you need project-specific instructions injected into a template.

## Development

This repo is a Cargo workspace:

- `crates/duckpond/` — core library shared by both binaries
- `crates/duckspec/` — CLI crate (binary: `ds`)
- `crates/duckboard/` — GUI crate (binary: `duckboard`)
- `crates/duckchat/` — agent-harness abstraction used by duckboard
- `crates/duckchat-claude-acp/` — owned ACP agent that wraps the official `claude` CLI
- `crates/duckchat-codex-acp/` — owned ACP agent that wraps official `codex app-server`

Harness agent binary discovery (first match wins):

| Harness | Env override | Sibling binary |
| --- | --- | --- |
| Claude | `DUCKCHAT_CLAUDE_ACP` | `duckchat-claude-acp` |
| OpenAI Codex | `DUCKCHAT_CODEX_ACP` | `duckchat-codex-acp` |

Then `PATH`. Sibling means next to the running `duckboard` executable.

- **DMG / `just bundle`:** agents are copied next to `duckboard` inside the app
  (primary GUI path; no separate install).
- **Local dev / `cargo run`:** build agents into the same `target/` tree so sibling
  resolution works:

```sh
cargo build -p duckchat-claude-acp -p duckchat-codex-acp -p duckboard
# or a full workspace build
cargo build
```

That places `target/debug/duckboard` next to the agent binaries (same under
`target/release/`). Override with `DUCKCHAT_CLAUDE_ACP=…` or `DUCKCHAT_CODEX_ACP=…`
when needed.

Common tasks via [just](https://github.com/casey/just):

```sh
just install       # install ds + duckboard + harness agents to ~/.cargo/bin
just bundle        # build dist/Duckboard.app (includes sibling agents)
just bundle-dmg    # build dist/Duckboard-<version>.dmg
just release 0.2.0 # bump workspace version, commit, tag, push → triggers CI release
```

Version control uses [jujutsu](https://github.com/martinvonz/jj) with a git backend; see `AGENTS.md` for conventions.

## License

MIT
