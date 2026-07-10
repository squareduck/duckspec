# Change status uses source backlinks

Make `ds status <change>` treat a change-introduced `test:code` scenario as linked when a
resolving source `@spec` backlink exists — not when the spec marker path list is non-empty
— so progress stops false-flagging good links as missing.

## Motivation

Change status is a progress view for work in flight. Today it decides coverage only from
scenario-level `> - path:line` entries on the `test: code` marker. The real link in this
system is the source `@spec` comment: when that resolves (and change-scoped audit is
green), the scenario is linked. Status still prints it under "missing," which misleads
agents mid-change and undermines trust in the dashboard.

## Scope

```text
caps/
├── audit/
│   ├── change-progress/   (untouched — audit gate stays)
│   └── scan-boundary/     (untouched — scan rules unchanged)
└── status/                 ← NEW
    └── change-coverage/    ← NEW
        └── spec.md
```

### New capabilities

- `status/change-coverage` — for `ds status <change>` only: coverage of change-introduced
  `test:code` scenarios is linked vs open based on resolving source backlinks

### Modified capabilities

- none

### Out of scope

- global `ds status` (project overview)
- `ds audit` and `audit/change-progress` (pending vs error classification, exit codes)
- step-checkbox claim classification
- turning status into a full integrity report
- duckboard UI (defer unless it already reuses this CLI path)

## Impact

```text
ds status <change>
        │
        ▼
  change-introduced test:code scenarios
  + source @spec scan (same kind of resolution as audit, not run_audit)
        │
        ▼
  progress: linked N / open M
  (linked ⇒ never listed as missing)
```

- Primary code: `crates/duckspec/src/cmd/status.rs`, plus a small duckpond helper for
  change scenarios × backlink keys if extraction keeps the CLI thin

- No intentional change to audit CLI or exit semantics
