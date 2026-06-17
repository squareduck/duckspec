# Post-ducknest salvage list

Daemon-free improvements that were lost when the project was rolled back to
the pre-ducknest commit (`swpvsnzqsnql`, *"feat(duckboard): add Files explorer
to changes area"*), and are worth re-implementing one at a time.

The ducknest daemon, the Telegram bot, and the daemon-backed duckboard state
layer were intentionally discarded and must **not** be re-implemented. This
entry captures only the genuinely useful library/CLI/GUI work that happened to
land during the ducknest era. Several of those commits were mixed (a single
"telegram" commit also carried a core `duckpond` refactor), which is why the
work has to be reconstructed by hand rather than cherry-picked.

## Status

The original list had five items. Items 1–4 (the `duckpond`/`duckspec`
audit / archive / merge cluster) were triaged against the current code and
folded into the **`spec-graph-integrity`** change — see *Folded into
`spec-graph-integrity`* below for what survived triage and what didn't. **Item
5 (GUI auto-scroll) remains the only standalone salvage item** and is preserved
in full.

## Provenance and how to read this

Each item lists the original change IDs from the discarded ducknest stack. Those
commits are unreferenced but still recoverable from `jj op log` (until garbage
collection), so `jj diff -r <id>` will show the original implementation as a
reference — *if* it's still around. The prose descriptions are written to be
sufficient on their own, so re-implementation does not depend on the diffs
surviving.

## Folded into `spec-graph-integrity`

Items 1–4 were validated against the post-rollback code before being scoped into
a single change. The triage corrected the original framing in two important
ways — recorded here so the reasoning isn't lost:

- **The unifying insight:** `audit_full` already computes the integrity
  invariants (it resolves every `@spec` backlink into `UnresolvedBacklink`). The
  refactor is about making the *mutation paths* (delta merge) and the *scan
  boundary* reuse that engine consistently — not about building new detection
  machinery. Implementation order is **3 → 2 → 1**: fix the live scan bug first,
  then the archive guard that reuses the resolver, then the merge consolidation.

- **Item 1 — validated merge entry point.** The original claim that "≥1 path
  validates a *doc* merge with the *spec* parser" is **false in the current
  code**: all three `apply_delta` call sites (`status::delta_new_coverage`,
  `audit::build_change_scenarios`, `archive::execute_plan`) either handle specs
  only with `parse_spec`, or — in archive — dispatch correctly by kind. That bug
  was ducknest-era and died in the rollback. What *is* real: the three sites
  handle merge results three different ways, and `merge.rs`'s own doc-comment
  ("caller should re-parse to validate") is honored by none consistently. So
  item 1 is a **consistency refactor, not a bugfix**. Scope: a single validated
  merge path (`merge_spec_delta` / `merge_doc_delta`, or one `merge_and_validate`
  keyed on artifact kind) plus `summarize_errors`, routing all three callers
  through it. **Decision (confirmed):** the silent swallowing in `status` and
  `audit` — they currently drop both merge and parse failures and emit empty
  results — must become **surfaced errors**, not silent no-ops.

- **Item 2 — archive orphan guard.** Real gap (no archive-time guard exists) but
  the original ~1.1k-line three-layer proposal is **oversized**: the detection
  primitive already exists. Archiving a capability with live backlinks is *not*
  silent drift — the next `ds audit` flags the now-unresolved backlinks. The
  guard is the *proactive* version of an invariant the audit already enforces
  reactively. Scope: project the post-archive scenario index, run the **existing**
  backlink resolver against it, and refuse / loudly warn when archiving would
  orphan live backlinks (reusing `UnresolvedBacklink` / `ScenarioKey`). No new
  audit primitive.

- **Item 3 — `exclude` config + nested-project skipping.** Fully valid and
  **live**: `ds audit` at the repo root currently reports **13 false-positive
  unresolved backlinks** and zero real ones. Breakdown — 9 from nested duckspec
  fixtures (`crates/duckspec/tests/fixtures/{good,bad}-project`), fixed by
  skipping any directory that owns its own `duckspec/caps/`; 4 from individual
  files (`crates/duckpond/tests/audit.rs`, `references/duckspec.md` code
  examples), fixed by an `exclude` key in `config.toml`. Both mechanisms are
  independently justified by present errors. Highest-value, lowest-risk piece —
  hence it leads the sequence. Implement the skip via the walker's `filter_entry`
  keyed on `duckspec/caps/` (not a bare `duckspec/` dir, to avoid colliding with
  the `crates/duckspec/` source crate); parse `exclude` as a `Vec<PathBuf>` in
  `duckpond::config` alongside `test_paths`, with a `ConfigError::BadExclude` for
  a non-array value and canonicalize-then-`starts_with` skipping during the scan.

- **Item 4 — CLI refinements.** Dissolves into item 1: it was "the CLI-side
  adoption of item 1 plus reporting polish." Once `status` / `audit` / `archive`
  route through the validated merge path, the adoption is done; any leftover
  wording tweaks are picked up opportunistically. No longer a discrete item.

Original ducknest refs, retained for diff-reading:
`pzmtpkmopkrs`, `puptmtpxvrtx` (item 1/4 merge + archive),
`olzpkxxpzuox` (item 2 audit orphan + tests, `audit_orphan.rs` /
`audit_broken_cap.rs`), `nvoxovppmzoq` (item 3 exclude + nested skip).

## Remaining standalone item

### Edge auto-scroll past the viewport for drag-selection

`refs: usuuwrqzkryx (duckboard/src/widget/autoscroll.rs +90, widget/text_edit/render.rs +185, widget/terminal.rs +89, widget/text_edit/state.rs +12, area/interaction.rs +17, main.rs +75)`

**Behavior.** When the user drags a text selection past the top or bottom edge of
a viewport, the view scrolls on its own so the selection keeps extending beyond
what's visible. Scroll speed ramps with how far past the edge the pointer sits —
a small overshoot creeps, a large one races, capped so it stays controllable.
Applies to both the text editor and the embedded terminal.

**Shape.**

- A self-contained pure function `widget::autoscroll::edge_velocity(pointer_y,
  top, bottom) -> f32` with `BASE` / `RAMP` / `MAX` constants: returns 0 inside
  the viewport, a signed per-frame velocity (positive = scroll content up /
  pointer past bottom) that ramps linearly with overshoot and clamps to `MAX`.
  Ships with its own unit tests; zero coupling to anything else.
- The text editor consumes it as pixel scroll; the terminal converts it to a line
  count.
- A subscription tick (~60fps, `std::time::Duration::from_millis(16)`) is added
  that fires **only** while a drag holds the pointer past an edge, advancing the
  scroll one frame at a time so it keeps moving even when the mouse is still.
- For chat messages that can't scroll themselves (selection ran past the chat
  fold), add a `pending_chat_autoscroll: Option<f32>` field on the session; the
  message accumulates the requested delta and `main` drains it into an absolute
  `scroll_to` on the outer chat scrollable after dispatch.

All the structures this hooks into (`state.interactions` → `sessions` /
`terminals`, `last_chat_offset_y`, `stick_to_bottom`, the scroll-preservation
replay) already exist on the current pre-ducknest board, so this is additive.

## Explicitly out of scope — do not re-implement

These were discarded on purpose and should stay gone: the ducknest daemon
(client/server, unix-socket protocol, sessions-in-daemon, lifecycle
reconciliation), the Telegram bot in all its forms, the daemon-backed duckboard
state layer (`nest.rs`, the disconnected-interaction modal, daemon emit-order,
keep-sessions-alive-across-scope-moves), and the global/project-less workspace
plus promotion. Two things that looked salvageable but need no work because they
already exist on the pre-ducknest board: the predicted next-command stage ladder
(the daemon-era `predict.rs` only *extracted* logic the board already has inline)
and model selection / slash-command listing (`duckchat` already exposes
`list_models` / `list_commands`).
