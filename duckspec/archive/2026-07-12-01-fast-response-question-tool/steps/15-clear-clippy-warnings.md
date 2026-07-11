# Clear clippy warnings

Clear clippy noise in `duckchat`, `duckchat-claude-acp`, and `duckboard` so those packages
build clean under clippy with tests. No new product behavior.

## Context

Followup `reviews/05-followup-awaiting-chips-and-clippy.md` issue 2. Known noise: collapsed
ifs, MutexGuard held across await, redundant closure, complex type.

## Tasks

- [x] 1. Fix clippy in `crates/duckchat` (lib + tests)

- [x] 2. Fix clippy in `crates/duckchat-claude-acp` (bin + tests)

- [x] 3. Fix clippy in `crates/duckboard` (bin + tests)

- [x] 4. Confirm `cargo clippy -p duckchat -p duckchat-claude-acp -p duckboard --tests` is
         clean (no warnings)
