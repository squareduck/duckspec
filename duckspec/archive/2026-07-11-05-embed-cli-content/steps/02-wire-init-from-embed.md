# Wire init from embed

Install harness command files from the embedded stock content instead of copying from the
build-tree `content/commands` path.

## Prerequisites

- [x] @step embed-content-and-wire-template-schema

## Tasks

- [x] 1. Switch `cmd/init.rs` to write harness commands via `content::command_files` /
         `fs::write`; drop `COMMANDS_DIR` and `fs::copy` from the source tree; keep the
         static harness allow-list and install paths

- [x] 2. Add integration coverage under `crates/duckspec/tests/` for init install and
         unknown harness (temp cwd, `CARGO_BIN_EXE_ds`)

- [x] 3. @spec cli/stock-content Stock content from the binary: Known harness commands are installed under the harness path

- [x] 4. @spec cli/stock-content Clear unknown-name failures: Unknown harness is rejected by name
