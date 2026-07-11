# Kinded catalog and discovery cleanup

Add `SlashCommandKind`, stop injecting Claude interactive builtins into shared discovery,
and build the completion catalog by merging duckboard's system registry with harness
discovery.

## Tasks

- [x] 1. Add `SlashCommandKind` (`System` / `Workflow` / `Agent`) to `SlashCommand` in
         `crates/duckchat/src/provider.rs` and re-export from `duckchat`; update every
         construction site to set a kind

- [x] 2. Remove the hard-coded Claude interactive builtins block from
         `crates/duckchat/src/claude_code/discover.rs` so shared discovery only returns
         filesystem skills/commands

- [x] 3. Add a duckboard system command registry (v1: `help` only) and a pure
         `build_completion_catalog(system, discovered)` helper that tags `ds-*` as
         Workflow, other discovered as Agent, and keeps System on name collision

- [x] 4. Wire catalog build where `chat_commands` is assigned (merge system registry after
         `list_commands`)

- [x] 5. @spec chat/slash-commands Kinded completion catalog: System registry entries are System

- [x] 6. @spec chat/slash-commands Kinded completion catalog: Discovered ds-* names are Workflow

- [x] 7. @spec chat/slash-commands Kinded completion catalog: Other discovered names are Agent

- [x] 8. @spec chat/slash-commands Kinded completion catalog: System name wins on collision with discovery

- [x] 9. @spec chat/slash-commands Kinded completion catalog: Claude interactive builtins are not Agent catalog entries
