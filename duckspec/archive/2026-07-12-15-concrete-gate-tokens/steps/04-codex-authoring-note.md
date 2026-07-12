# Codex authoring note

Record the durable write-gate token principle so future templates do not reintroduce bare
`confirm` / `reject`.

## Prerequisites

- [x] @step style-gate-token-rules

## Tasks

- [x] 1. Add a short principle to `duckspec/codex/template-and-schema-authoring.md`:
         write-gate send tokens name the decision; reasons only on slash-command handoffs

- [x] 2. `ds format` and `ds check` the codex entry (and reinstall/`cargo build` note for
         embedded stock content if verifying CLI output)
