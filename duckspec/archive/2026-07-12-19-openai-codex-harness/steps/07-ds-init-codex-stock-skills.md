# ds init codex stock skills

Ship stock stage skills for Codex and install them under `.agents/skills` via
`ds init codex`.

## Prerequisites

- [x] @step duckboard-registration-and-packaging

## Tasks

- [x] 1. Add stock content `crates/duckspec/content/commands/codex/<stage>/SKILL.md` for
         each stage (same stage set as claude/opencode)

- [x] 2. Extend `content` + `init` to support skill-directory install for harness `codex`
         → `.agents/skills`

- [x] 3. Update CLI help / harness list to include `codex`

- [x] 4. @spec cli/stock-content Stock content from the binary: Known codex skills are installed under .agents/skills

- [x] 5. Confirm re-init overwrites stock skill bodies and unknown harness still fails by
         name
