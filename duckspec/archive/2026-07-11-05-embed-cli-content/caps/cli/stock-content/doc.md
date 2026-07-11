# Stock CLI content

Stock harness commands, agent templates, and schema guides ship inside the `ds` binary and
drive `init`, `template`, and `schema` without a runtime source tree.

## What is stock content

Three kinds of fixed text ship with every `ds` build:

```
| Kind       | Consumed by          | Role                                              |
| ---------- | -------------------- | ------------------------------------------------- |
| Commands   | `ds init <harness>`  | Slash-command stubs installed into a project      |
| Templates  | `ds template <name>` | Full agent stage prompts printed to stdout        |
| Schemas    | `ds schema <name>`   | Artifact and style guides printed to stdout       |
```

Authors edit these files under the crate's `content/` tree at development time. At runtime
the binary already holds their bodies; a release install does not need that tree on disk.

## Commands

`ds init` (no harness) only ensures the `duckspec/` skeleton exists under the current
working directory.

`ds init <harness>` also installs stock command files for a supported harness into a
harness-specific project directory:

```
| Harness  | Install path           |
| -------- | ---------------------- |
| claude   | `.claude/commands/`    |
| opencode | `.opencode/commands/`  |
```

Each installed file is a small markdown stub agents load as a slash command (typically
invoking `ds template <stage>`). Re-running `ds init <harness>` overwrites those files
with the stock bodies from the current binary.

## Templates and schemas

`ds template <name>` prints one stock stage template (`explore`, `propose`, `design`, and
the rest of the workflow set). Project hooks under `duckspec/hooks/` can still inject
before/after sections into a rendered template; hooks remain ordinary project files on
disk.

`ds schema <name>` prints one stock guide (`proposal`, `design`, `spec`, `style`, and
related artifact shapes). Schemas are not project-local overrides; they are fixed
references for agents and authors.

## Unknown names

A request for a template, schema, or harness that is not stock fails with an error that
names the unknown value. Callers should treat that as a bad name, not as a broken install
or missing checkout.
