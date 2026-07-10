# Session field and soft hint

Replace `AgentSession.obvious_command` with `obvious_chrome`, refresh from
disk/session/VCS, and keep oneshot soft hint + orientation on lifecycle[0].

## Prerequisites

- [x] @step phase-builder-and-composition

## Tasks

- [x] 1. Replace `obvious_command: Option<String>` with `obvious_chrome: ObviousChrome` on
         `AgentSession` (default empty)

- [x] 2. Implement `refresh_obvious_chrome` (replace `refresh_obvious_command`) using
         per-session emptiness and a shared `vcs_dirty` flag; update all call sites
         (change area, ideas, main, archive migration paths)

- [x] 3. Re-refresh when transcript emptiness or `changed_files` dirtiness changes so gate
         row and Commit appear/disappear correctly

- [x] 4. Soft-hint oneshot construction from `lifecycle[0]` / `scope_facts.next_command`
         only; keep `default_prompts` list oneshot-parse-only

- [x] 5. Fix compile breaks and unit tests that still read `obvious_command`
