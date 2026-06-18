# duckspec

Spec-driven development framework for AI coding agents — a Rust core library, a CLI, and a
macOS desktop companion that all operate on a single `duckspec/` directory committed
alongside your code.

## What it is

duckspec gives a coding agent a structured workflow for the whole lifecycle of a change:
explore, propose, design, specify, implement, and verify. Behavior is captured in paired
spec/doc capability files, and source tests link back to the scenarios they verify — so
spec drift is detectable instead of silent.

## Components

- **duckpond** — core library; the authoritative parser, validator, and engine.

- **ds** — CLI over duckpond; scaffolds projects, audits integrity, and renders the agent
  slash-command templates.

- **duckboard** — macOS GUI over the same `duckspec/` files.

- **duckchat** — agent-harness abstraction duckboard uses to drive coding agents.

## Principles

- **Filesystem is the source of truth.** No frontmatter, no sidecar metadata.
- **Library first.** duckpond owns the logic; ds and duckboard are thin views over it.
- **Spec drift is a build-time error**, not a review-time hope.
- **Typed errors in the library** (`thiserror`); `anyhow` at the binary boundary.
