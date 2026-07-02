# Capture the originating session

Record `slug → exploration id` when an exploration session's agent runs
`ds create change`, by parsing the Bash tool call as it streams into the
`AgentEvent::ToolUse` handler. Implements the design's *Pending-binding store*, *Command
parser*, and *Capture hook* components.

## Tasks

- [x] 1. Add `pending_bindings: HashMap<String, String>` (folder-slug → exploration id) to
         `area::change::State` in `crates/duckboard/src/area/change.rs`, and initialize it
         empty wherever the struct is constructed (`Default`/`new`).

- [x] 2. Implement `extract_create_change_arg(command: &str) -> Option<String>` in
         `crates/duckboard/src/main.rs`: locate the `ds create change` invocation, return
         the next shell token (handling a single quoted multi-word argument), stopping at
         a shell separator (`&&`, `;`, `|`, newline); return `None` when absent.

- [x] 3. Implement `parse_create_change(name: &str, input: &str) -> Option<String>`: guard
         on `name == "Bash"`, extract `.command` from the tool-input JSON via
         `serde_json`, run `extract_create_change_arg`, and return
         `duckpond::slug::slugify(&arg)`.

- [x] 4. Wire capture into the `AgentEvent::ToolUse` arm (`main.rs:1368`): when
         `ax.scope_kind == scope::ScopeKind::Exploration` and `parse_create_change` yields
         a slug, stage `(slug, ax.session.scope.clone())` in a local; commit
         `state.change.pending_bindings.insert(...)` after the `ax` borrow ends, alongside
         the existing `title_task_input` staging.

- [x] 5. Unit-test `parse_create_change` / `extract_create_change_arg`: plain
         `ds create change my-thing`, quoted multi-word title, `cd`-prefixed and
         `&&`-compound command, non-`Bash` tool name, and a Bash command without
         `ds create change` (each asserting the expected `Some(slug)` / `None`).
