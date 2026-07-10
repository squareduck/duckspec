# Cmd-Enter obvious bubble

Restore instant lifecycle “next step” via a greyed ghost user bubble and ⌘↩, while
composer empty-input suggestions become LLM-only. When all steps are complete, the
lifecycle command is `/ds-archive` (finalize), not `/ds-review`.

## Motivation

Oneshot reply suggestions reintroduced multi-second latency on the empty-composer path.
Structural next stages are already known from disk; they should never wait on a model.
Seeding the composer list from the lifecycle heuristic mixed two sources and still blocked
empty Enter while the oneshot was pending. Splitting surfaces restores speed without
letting oneshot results pollute the hotkey path.

## Scope

```
caps/chat/
├── default-prompts/   (modified — list = oneshot parse only)
└── obvious-bubble/    ← NEW (ghost + ⌘↩)
session/
└── scope/             (modified — steps complete → archive)
```

### New capabilities

- `chat/obvious-bubble` — When idle (not streaming), the composer is empty, and a
  lifecycle `obvious_command` is present, show a greyed faux user bubble with that command
  in empty-send form (e.g. `/ds-explore`) and a ⌘↩ hint. ⌘↩ or bubble activation sends
  that exact text as a real user message, replacing the ghost. The bubble is never driven
  by oneshot `REPLY:` lines. No bubble when the command is absent (caps, codex, archived).

### Modified capabilities

- `chat/default-prompts` — Effective composer list is only a settled non-empty oneshot
  parse. No pre-oneshot heuristic list and no fail/empty fallback to the heuristic. The
  lifecycle heuristic remains a soft oneshot request hint only. Empty Enter and Tab cycle
  that list alone.

- `session/scope` — When all steps are complete, the lifecycle next stage is archive
  (`ds-archive`), not review. Orientation and the soft oneshot hint follow the same
  ladder; agents may still propose review via handoff or oneshot text.

### Also (no new capability)

- Workflow template Handoffs: reword `**Primary**` / `**Secondary**` to a flat “Suggested
  next actions:” bullet list. Ranked order and the ≤2 rule stay the same.

### Out of scope

- Freeform hotkeys (commit messages, create-change names)

- Empty Enter sending the heuristic while an oneshot is pending

- Dual agent vs hotkey command fields (single `obvious_command`; only the steps-complete
  value changes)

- Changing apply/review handoff *policy* (review-before-archive may remain in agent prose)

- Lifecycle hotkeys on caps or codex scopes

## Impact

```
disk phase ──▶ obvious_command ──▶ ghost + ⌘↩
                    │
                    └── soft hint ──▶ oneshot ──▶ REPLY: ──▶ composer list
```

- duckboard stops seeding `agent_default_prompts` from the heuristic; adds bubble UI and
  ⌘↩; ladder tweak in `change_scope_facts` (steps complete → archive)

- Spec/tests for default-prompts scenarios that currently require a heuristic list or
  fallback

- Orientation for “all steps complete” reports archive

- Template handoff wording under `crates/duckspec/content/templates/` only
