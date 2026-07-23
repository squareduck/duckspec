# Rename hook create to before/after

Hard-break plan and CLI create from `pre`/`post` to `before`/`after`, with tests for
scaffold, refusals, and known stages.

## Tasks

- [x] 1. Rename `HookPosition` to `Before`/`After`, `as_str` → `"before"`/`"after"`, and
         update `create_hook` + plan unit tests

- [x] 2. Switch `create.rs` to `--before`/`--after`, skeleton `# {Stage} - Before|After`,
         and help text matching `STAGES`

- [x] 3. @spec cli/hooks Create hook: Scaffold before and after paths with skeleton

- [x] 4. @spec cli/hooks Create hook: Refuse when the hook file already exists

- [x] 5. @spec cli/hooks Create hook: Refuse without exactly one of before or after

- [x] 6. @spec cli/hooks Known stages: Unknown stage is rejected
