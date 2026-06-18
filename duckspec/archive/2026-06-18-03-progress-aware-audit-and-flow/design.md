# Progress-aware change audit and workflow guidance — Design

Carry step checkbox state into `duckpond::audit::audit_change` so unlinked `test:code`
scenarios split into a *pending* bucket (no checked step task) and an *error* bucket
(checked but unlinked), then render and exit on that distinction and rewrite the affected
agent templates around it.

## Approach

```
ds audit <change>
        │
        ▼
  audit_change()                       caps/, change spec/deltas
        │                                       │
        ├─ build change_scenarios ◄─────────────┘
        ├─ scan_source_files ──► backlink_keys      (// @spec comments)
        ├─ collect_step_refs  ──► claimed set       (refs on CHECKED tasks)
        │
        ▼
  for each test:code change scenario with no backlink:
        │
        ├─ key ∈ claimed  ──►  missing_backlink_scenarios   (×  error, exit 1)
        └─ key ∉ claimed  ──►  pending_backlink_scenarios   (·  info,  exit 0)
        │
        ▼
  AuditReport ──► CLI print_report (two sections) ──► total_errors() drives exit
```

The whole change pivots on one fact already in the tree: step `Task`/`Subtask` carry
`checked: bool` (`crates/duckpond/src/artifact/step.rs:37`, `:45`), and
`collect_step_refs` already parses every `@spec` task — it just discards the checkbox. We
thread that bit through, add an additive report bucket, and classify. Full audit
(`audit_full`) is untouched: its missing-backlink set comes from archived `caps/`
scenarios where no step context exists, so those stay hard errors.

## StepRef carries checkbox state

`collect_step_refs` (`crates/duckpond/src/audit.rs:1064`) gains one field on the internal
`StepRef` and copies the checkbox from the ref-bearing item (the task or subtask that
holds the `@spec`, not its parent).

```rust
struct StepRef {
    key: ScenarioKey,
    step_file: PathBuf,
    line: usize,
    checked: bool, // NEW: from task.checked / sub.checked
}
```

In the collection loop, `task.checked` is used for a task-level `SpecRef` and
`sub.checked` for a subtask-level `SpecRef`. No other caller of `collect_step_refs` reads
the new field, so this is backward compatible.

## AuditReport gains a pending bucket

`missing_backlink_scenarios` keeps its meaning — genuine errors — and a new sibling field
holds the informational pending set. Additive, so existing consumers (the `ds` CLI,
`duckboard`) keep compiling and `total_errors()` is unchanged.

```rust
pub struct AuditReport {
    // ...
    /// test:code scenarios with no source backlink whose step task is CHECKED:
    /// implementation claimed done but the backlink is missing. An error.
    pub missing_backlink_scenarios: Vec<ScenarioKey>,
    /// test:code scenarios with no source backlink that are NOT yet claimed
    /// (no checked step task). Informational — not counted by total_errors().
    pub pending_backlink_scenarios: Vec<ScenarioKey>,
    // ...
}
```

`total_errors()` (`audit.rs:70`) is left as-is — it already sums
`missing_backlink_scenarios` and never sees the pending bucket, so a change with only
pending scenarios reports zero errors and exits 0.

## Classification in audit_change

`collect_step_refs` is hoisted above the missing-backlink loop (it is currently called
lower, at `audit.rs:489`, for the coverage check — the same vector is reused). A "claimed"
set is built from refs on checked tasks; a scenario is treated as claimed-implemented when
*any* referencing step task is checked.

```rust
let step_refs = collect_step_refs(duckspec_root, canonical_root, change_name)?;

let claimed: HashSet<&ScenarioKey> =
    step_refs.iter().filter(|r| r.checked).map(|r| &r.key).collect();

for s in &change_scenarios {
    if s.test_code && !backlink_keys.contains(&s.key) {
        if claimed.contains(&s.key) {
            report.missing_backlink_scenarios.push(s.key.clone()); // error
        } else {
            report.pending_backlink_scenarios.push(s.key.clone()); // pending
        }
    }
}
report.pending_backlink_scenarios.sort_by_key(|k| k.display());
```

The existing coverage check (`audit.rs:483`) and step-ref resolution (`audit.rs:505`)
reuse the same `step_refs` vector below, unchanged.

## ds audit CLI rendering

`print_report` (`crates/duckspec/src/cmd/audit.rs:78`) renders the two buckets distinctly.
Errors keep the red `×` with clarified wording; pending uses the dimmed `·` it uses
elsewhere for progress notes.

```rust
for key in &report.missing_backlink_scenarios {
    eprintln!("  {} {} — step task checked off but no backlink resolves",
        "×".red(), key.display());
}
for key in &report.pending_backlink_scenarios {
    eprintln!("  {} pending: {} — not yet implemented",
        "·".dimmed(), key.display());
}
```

The exit path (`audit.rs:38`) is unchanged — it keys off `total_errors()`, which excludes
pending. The success line gains a pending note when applicable, e.g.
`✓ audit ok — N scenario(s) pending implementation`, so a clean-but-incomplete change
reads as in-progress rather than done.

## Template and schema edits

Freeform edits under `crates/duckspec/content/`, each seeding the same flow:

```
spec → step → apply (×N) ──► ds audit <change> after each (progress)
                                     │
                          all clean (0 pending, 0 error) = archive-ready
                                     │
                          ds archive <name> ──► ds sync ──► ds audit (full)
```

- `templates/apply.md` — after `ds check <step-file>`, run `ds audit <change>`; explain
  pending (later steps, expected) vs error (this step's checked task is unlinked — fix
  before handoff). Wire into the existing "more steps / all done" handoff branches.

- `templates/step.md` — note the scoped audit belongs to `/ds-apply`; don't run it at step
  time.

- `templates/archive.md` — after `ds archive <name>`, run `ds sync` (stamp backlinks into
  the freshly-landed caps specs) then `ds audit` (full integrity). Today it mentions
  neither.

- `templates/verify.md` — distinguish `ds audit` (full, in-flight-tolerant) from
  `ds audit <change>` (per-change progress/completeness gate).

- `schemas/spec.md` — document the `test: code` marker, that source `@spec` comments are
  the backlinks the audit resolves, and that `ds sync` stamps the resolved `path:line`
  into caps markers.

## Decisions

- **Claimed = any referencing task checked** — chosen over "all referencing tasks
  checked". Checking off "implement scenario X as a test" is the agent asserting it is
  done; a missing backlink at that point is a real defect. Alternatives: all-checked
  (rejected — masks a finished single task behind an unrelated sibling); backlink-presence
  heuristics (rejected — that is what the scan already measures).

- **Classification only in scoped audit** — `audit_full` keeps pushing missing backlinks
  as hard errors. Its set comes from archived `caps/` scenarios with no step context, so
  the pending/error split has no input there.

- **Additive `pending_backlink_scenarios` field** — over reshaping
  `missing_backlink_scenarios` into `Vec<MissingBacklink { key, status }>`. The enum is
  tidier but changes a public field type and breaks `duckboard`'s rendering; an additive
  field keeps every existing consumer compiling and leaves error semantics exactly as they
  were.

- **Pending reported independently of coverage misses** — a scenario with no step task at
  all lands in pending (backlink) and is also flagged by the existing
  missing-step-coverage error. Kept independent: pending is the "todo" view, the coverage
  error explains *why* (no step yet). Alternative: suppress pending when also a coverage
  miss (rejected — couples two checks for marginal noise reduction).

- **Archive stays template-instructed, not auto-sync** — the proposal deferred
  auto-running `ds sync` inside `ds archive`. Resolved here as: keep it a template
  instruction in `archive.md` for now. Auto-sync changes archive's contract (it would
  write `caps/` specs as a side effect of archiving) and deserves its own change if
  wanted. Alternative: bake sync into `archive::run` (rejected for this change — wider
  blast radius, separate concern).

## Risks

- **Subtask vs task checkbox ambiguity** → use the checkbox on the item that actually
  holds the `@spec` ref (`task.checked` for task refs, `sub.checked` for subtask refs),
  never the parent's.

- **Split implementation across checked + unchecked tasks** → with any-checked, a scenario
  whose first task is checked but whose remaining work sits in an unchecked task reads as
  claimed, so a still-missing backlink surfaces as an error rather than pending.
  Acceptable and arguably correct; documented via the decision above.

- **CLI consumers reading only `missing_backlink_scenarios`** → they silently lose
  visibility of pending work. Mitigated by it being purely informational; `duckboard` can
  adopt the new field later without correctness impact.

## Open questions

- None. (Auto-sync-on-archive resolved as out of scope; see Decisions.)
