# Chat slash commands

Kinded slash-command catalog for chat completion, local system handlers (including
`/help`), and a double-slash escape so colliding agent skills stay reachable.

## Kinds

Every completion entry has one kind:

```
| Kind     | Meaning                                              | Example   |
| -------- | ---------------------------------------------------- | --------- |
| System   | Handled by duckboard; bare submit never starts agent | `/help`   |
| Workflow | Duckspec stage templates; sent to the agent          | `/ds-spec`|
| Agent    | Harness / project / plugin skills; sent as-is        | `/review` |
```

The catalog is the merge of a duckboard system registry and harness discovery. On a name
collision, System wins — one entry, kind System.

Claude interactive builtins (`clear`, `compact`, `cost`, `help`, `model`) are not injected
as Agent entries by shared discovery. System `help` comes only from the duckboard
registry.

## Submit routing

```
submit text
    │
    ├── bare system name   → local handler (no agent turn)
    ├── bare //name        → agent turn, prompt = /name, user text = //name
    └── anything else      → normal agent turn
```

v1 system surface is `/help` only.

## Local `/help`

Records a user message (`/help`) then a system message. Does not stream, prime, or consume
selection attachments.

System message shape:

1. Fixed prefix: running system command `/help`; agent help via `//help`
2. Sections from the live catalog by kind (System, Workflow, Agent) — omit empty sections
3. Escape note for `//help`

Agent section titles include the active harness id when present (e.g. Agent skills →
grok).

## Completion cues

```
| Kind     | Name color     | Tag   |
| -------- | -------------- | ----- |
| System   | system accent  | `sys` |
| Workflow | workflow color | —     |
| Agent    | agent color    | —     |
```

The three name colors are pairwise distinct. Fuzzy score still ranks matches; equal scores
break ties as System → Workflow → Agent.
