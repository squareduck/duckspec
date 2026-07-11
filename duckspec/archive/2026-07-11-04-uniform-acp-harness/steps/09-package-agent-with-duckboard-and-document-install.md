# Package agent with Duckboard and document install

Ship `duckchat-claude-acp` as a sibling of `duckboard` in the app bundle so
Finder-launched Duckboard does not need the agent on PATH. Document non-bundle install
(PATH / env) for cargo and curl-style layouts.

## Prerequisites

- [x] @step map-claude-thinking-to-thought-chunks

## Context

Review `03-review-uniform-acp-harness-post-implementation-review` finding 1. Discovery is
env → sibling of exe → PATH. GUI apps often have a skeletal PATH, so the primary product
path is bundling the agent next to `duckboard` in `Contents/MacOS/`. PATH install remains
a valid fallback for terminal/dev use.

## Tasks

- [x] 1. Update `justfile` `bundle` to
         `cargo build --release -p duckchat-claude-acp
                       -p duckboard` and
         copy `target/release/duckchat-claude-acp` into `Contents/MacOS/` beside
         `duckboard`.

- [x] 2. Ensure release CI packages the agent with the app
         (`.github/workflows/release.yml` and/or the bundle recipe used by `bundle-dmg`).

- [x] 3. Update README install/run docs: DMG/app expects a sibling agent (no separate
         install); note `just install` / cargo / PATH and `DUCKCHAT_CLAUDE_ACP` for
         non-bundle layouts.

- [x] 4. Confirm `just install` installs `ds`, `duckboard`, and `duckchat-claude-acp`
         (already listed — fix if incomplete).

- [x] 5. Local smoke: after `just bundle` (or equivalent), agent binary is present next to
         duckboard under the app bundle MacOS dir.
