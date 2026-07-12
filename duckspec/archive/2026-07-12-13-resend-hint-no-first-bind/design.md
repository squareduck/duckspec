# Resend-history hint without first-bind flash - Design

Narrow the composer resend-history hint so it keys off a **stored but unresumable** agent
session id (harness mismatch), not “no resume id yet.” Pure helper + footer wiring only;
preamble and recovery stay unchanged.

## Approach

Today the footer conflates two “can’t resume” states:

```
                    has messages?
                         │
            ┌────────────┴────────────┐
            no                       yes
            │                         │
         hidden              will_resume?  (= resumable_session_id.is_some())
                         ┌────┴────┐
                        yes       no
                         │         │
                      hidden    SHOWN  ← includes first-bind (no id yet)
```

Target:

```
                    has messages?
                         │
            ┌────────────┴────────────┐
            no                       yes
            │                         │
         hidden         stored agent_session_id?
                         ┌────┴────┐
                        no       yes
                         │         │
                      hidden   resumable for effective harness?
                                ┌────┴────┐
                               yes       no (foreign / durable unresumable id)
                                │         │
                             hidden     SHOWN
```

```
| State | `agent_session_id` | `resumable_session_id` | Hint |
| --- | --- | --- | --- |
| First bind / mid first turn | `None` | `None` | hidden |
| Post-recovery (id cleared) | `None` | `None` | hidden |
| Normal resume | `Some` | `Some` | hidden |
| Harness switch (foreign id) | `Some` | `None` | **shown** |
```

Send-path preamble (`interaction.rs` ~3202) keeps
`resumable_session_id().is_none() && !messages.is_empty()` — so recovery and
unbound-with-history still re-feed; only the **advertisement** narrows.

## Pure hint helper

`crates/duckboard/src/widget/agent_chat.rs` — replace the two-arg “not resume + messages”
rule.

```rust
/// Footer resend-history hint: non-empty transcript and a stored agent
/// session id that is not resumable on the effective harness.
pub fn show_resend_history_hint(
    has_messages: bool,
    unresumable_stored_session: bool,
) -> bool {
    has_messages && unresumable_stored_session
}
```

`unresumable_stored_session` means:
`agent_session_id.is_some() && resumable_session_id().is_none()` — computed at the wire
site, not re-derived inside the widget.

Unit tests under the same module update to the three outcomes above (switch/show,
resume/hide, no-id/hide, empty/hide). Keep `@spec` backlinks on the revised
`chat/composer-footer` scenarios.

## StatusInfo + view wire

`StatusInfo.will_resume` is only used for this hint. Rename/repurpose to match the new
signal:

```rust
pub struct StatusInfo {
    // …
    /// Stored agent id exists but is not resumable for the effective harness
    /// (typically after a harness switch). False when unbound or when resume works.
    pub unresumable_stored_session: bool,
    // …
}
```

Wire in `crates/duckboard/src/area/interaction.rs` (status construction ~4279):

```rust
unresumable_stored_session: ax.session.agent_session_id.is_some()
    && ax.resumable_session_id().is_none(),
```

View call site (~1474):

```rust
if show_resend_history_hint(
    !session.messages.is_empty(),
    status.unresumable_stored_session,
) { /* "⟳ resends full history" */ }
```

No change to `resumable_session_id()`, model picker, or `SessionIdUpdated` stamping.

## Capability delta

Delta (not a new cap) on live `duckspec/caps/chat/composer-footer/`:

- **Requirement** — hint only when transcript non-empty **and** a stored agent session id
  exists **and** it is not resumable for the effective harness; SHALL NOT appear for empty
  transcript, resumable id, or **no stored id** (first bind / post-recovery clear).

- **Scenarios** — keep resume + empty; rewrite “shown when history would be resent” to
  foreign/unresumable stored id; add **hidden when no stored agent session id** (non-empty
  transcript).

- **doc.md** — replace the 2-column resume table with the 3-signal table above.

## Impact

- Duckboard-only: `agent_chat` helper/tests, `StatusInfo` field rename, one wire site.
- Spec/doc delta under existing `chat/composer-footer`.
- No duckpond/CLI/API, no persistence shape change, no harness resume semantics change.

## Decisions

- **Signal = stored id ∧ ¬resumable** — not “¬will_resume alone.” Alternatives: (1) hide
  only while `is_streaming` on first turn (rejected: still wrong for any
  unbound-with-history idle frame, and couples hint to stream timing); (2) show for all
  non-resumable including no-id (status quo; rejected by proposal).

- **Rename `will_resume` on `StatusInfo`** — field is hint-only today; renaming avoids a
  dead “resume?” flag that no longer matches product meaning. Alternative: keep
  `will_resume` and add a second bool (rejected: two overlapping booleans at the view).

- **Preamble/recovery untouched** — advertising ≠ send behavior; recovery may still
  re-send silently (proposal non-goal + intent).

- **Same-harness model change** — still resumes; hint stays hidden (proposal non-goal).

## Risks

- **Callers forget the rename** → compile error on `StatusInfo` field (single construction
  site; low).

- **Legacy sessions with id but missing `session_harness`** → owner defaults to
  `claude-code` in `resumable_session_id`; foreign only when effective harness ≠ that.
  Unchanged resume semantics; hint follows the same rule.
