# @ Stock CLI content

## ~ Commands

`ds init` (no harness) only ensures the `duckspec/` skeleton exists under the current
working directory.

`ds init <harness>` also installs stock agent stage content for a supported harness into a
harness-specific project directory:

```
| Harness  | Install path           | Layout                                      |
| -------- | ---------------------- | ------------------------------------------- |
| claude   | `.claude/commands/`    | Flat `ds-*.md` command stubs                |
| opencode | `.opencode/commands/`  | Flat `ds-*.md` command stubs                |
| codex    | `.agents/skills/`      | Skill dirs: `<stage>/SKILL.md` per stage    |
```

Claude and OpenCode install small markdown stubs agents load as slash commands (typically
invoking `ds template <stage>`). Codex installs the same stage set as Agent Skills under
`.agents/skills`, which is where Codex discovers repo skills. Re-running
`ds init
<harness>` overwrites those files with the stock bodies from the current binary.
