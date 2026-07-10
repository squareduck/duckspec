# Orientation paths and workflow handoffs

Stop agents hunting for change dirs by giving project-root-correct paths in session
orientation, and make every workflow template hand off with a hard, ranked ≤2 next-action
list that matches the current lifecycle (review before archive, commit after archive).

## Motivation

Agents (especially Grok) often search for change artifacts because orientation says
`changes/{name}/` while the real path is `duckspec/changes/{name}/`. Non-change scopes
point at root files (`caps.md`, `codex.md`) that do not exist.

Separately, template handoffs are inconsistent: archive suggests nothing, apply jumps
straight to archive and skips review, and some stages offer three competing next steps.
Fixing both reduces thrash and aligns agent guidance with the intended workflow.

## Scope

```
caps/
└── session/
    └── scope/          (modified — paths + all-steps-complete next stage)
```

### Modified capabilities

- `session/scope` — Orientation paths are project-root-relative under `duckspec/`
  (`duckspec/changes/{name}/`; caps → `duckspec/caps/` + `duckspec/project.md`; codex →
  `duckspec/codex/` + `duckspec/project.md`). When all steps are complete, suggested next
  stage is `/ds-review` (not `/ds-archive`). Still no discovery dump in orientation —
  agents use `ds status` / `ds index` via templates.

### New capabilities

- None

### Out of scope

- New capability for template text / handoff policy (content-only updates under
  `crates/duckspec/content/templates/`)

- Removing or redesigning `/ds-verify` as a skill

- `/ds-followup` skill

- Auto-commit after archive (still propose message + wait)

- Changing write gates, schemas, or stage mechanics beyond handoff wording and orientation
  strings/facts

## Impact

```
first turn
  AgentsMarkdownHook + CurrentScopeHook  ──→  agent
       paths/next stage in scope.rs / change_scope_facts

workflow slash commands
  templates/*.md handoffs  ──→  ranked ≤2 next actions
       apply(done) → review → archive → commit
```

- Code: `crates/duckboard/src/scope.rs`, `crates/duckboard/src/area/change.rs`
  (`ChangeScopeFacts` / `next_command` when all steps complete), related unit tests and
  `session/scope` scenarios

- Content: handoff sections in explore, propose, design, spec, step, apply, archive
  (codex/backfill/review only if needed to enforce ≤2)

- Ranked handoff matrix (templates, not a capability):

```
  | Stage | Primary | Secondary |
  |-------|---------|-----------|
  | explore | create change (or `/ds-propose` if change exists) | `/ds-propose` (if create was primary) |
  | propose | `/ds-design` | `/ds-spec` |
  | design | `/ds-spec` | `/ds-step` |
  | spec | `/ds-step` | `/ds-archive` |
  | step | `/ds-apply` | — |
  | apply (open steps) | `/ds-apply` | — |
  | apply (all done) | `/ds-review` | `/ds-archive` |
  | archive | commit (proposed message; wait) | — |
```
