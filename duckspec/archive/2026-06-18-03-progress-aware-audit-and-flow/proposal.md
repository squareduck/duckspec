# Progress-aware change audit and workflow guidance

Make `ds audit <change>` use step checkbox state to tell not-yet-implemented scenarios
apart from genuinely broken backlinks, and update the agent command templates so the
audit/sync/archive flow is unambiguous.

## Motivation

Agents drive this workflow daily and misread change-scoped `ds audit`. After `/ds-step` it
flags every `test:code` scenario as "no backlink" — the expected pre-implementation state,
not a defect. Because `/ds-apply` completes one step at a time, those errors persist
across a multi-step change with no way to distinguish "not done yet" from "done but
mis-linked".

The templates compound it: they never teach scoped vs full audit, never instruct `ds sync`
(so caps coverage markers stay unstamped), and don't sequence sync after archive. The
noise erodes trust in the audit and leads agents to "fix" non-problems.

## Scope

```
caps/
└── audit/
    ├── scan-boundary/    spec.md   (unchanged)
    └── change-progress/   ← NEW
        └── spec.md
```

### New capabilities

- `audit/change-progress` — change-scoped audit classifies each missing-backlink
  `test:code` scenario as **pending** (its step task is unchecked or absent) vs **error**
  (its step task is checked, but no source backlink resolves). Only errors fail the audit;
  pending scenarios are reported informationally.

### Modified capabilities

- None. Full `ds audit` keeps its existing in-flight backlink exemption.

### Non-capability work

These ride along as freeform implementation tasks (no `@spec` backlinks):

- Templates `apply.md`, `step.md`, `archive.md`, `verify.md` — teach scoped vs full audit,
  the pending/error distinction, `ds audit <change>` as a per-apply progress check, and
  `ds sync` then full `ds audit` after archive.

- Spec schema doc — document `test: code` marker and backlink semantics, and that
  `ds sync` populates caps markers.

- `ds` audit CLI output — render pending vs error distinctly.

### Out of scope

- Auto-running `ds sync` inside `ds archive` (deferred — revisit in design).
- Any change to full `ds audit`'s in-flight backlink exemption.
- A batch/unscoped `ds archive` form.

## Impact

- Behavior change to `ds audit <change>` exit semantics: a change with only pending
  (unimplemented) scenarios no longer exits non-zero.

- CLI audit output gains a pending vs error split.

- No library API removed; full `ds audit` is unaffected.
