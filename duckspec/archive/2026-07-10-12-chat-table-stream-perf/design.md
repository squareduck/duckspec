# Chat stream UI stays responsive with tables — Design

Keep applying stream text to session state every event, but rebuild chat editors and
hybrid table layout on a bounded cadence (plus immediate rebuilds on structural events).
Share hybrid layout by `Arc` so settled blocks stop paying clone-on-hit every frame.

## Approach

```
AgentEvent
   │
   ├─ always: apply to ChatSession (pending_text / tools / …)
   │          mark chat_ui_dirty when transcript body may change
   │
   ├─ structural (Tool*, TurnComplete, kind switch, cancel/error)
   │          → materialize_chat_ui() immediately
   │
   └─ ContentDelta / ReasoningDelta
              → wait for StreamTick (or next structural)
                    │
                    ▼
           materialize_chat_ui()
             rebuild blocks; reuse settled editors
             in-place refresh live answer (append + partial highlight)
                    │
                    ▼
           TextEdit layout/draw
             hybrid cache: Arc<EditorLayout>  (clone Arc, not the tree)
```

**Strategy.** Split **session apply** from **UI materialize**. Today every `AgentEvent`
both mutates `ChatSession` and runs `rebuild_chat_editor` — full `lines.join` + syntect on
any block whose content changed. That is correct but main-thread-heavy once hybrid table
layout and full-buffer highlight sit on the live answer.

This change:

1. Always applies deltas to the session (no loss, no protocol change).

2. Materializes chat editors only when dirty **and** either a structural event arrives or
   the existing streaming `StreamTick` fires (~100 ms via `streaming_indicator::TICK_MS`).

3. Refreshes the live answer editor in place when block identity is stable, instead of
   `EditorState::new` + full re-highlight every tick.

4. Shares hybrid layout geometry through `Arc<EditorLayout>` so `layout` / `update` /
   `draw` do not deep-clone table regions on cache hits.

`editor/md-table` and `chat/transcript` stay as-is. No list virtualization in this change.

## Stream apply vs materialize

Session mutation stays in `main`’s `AgentEvent` match. UI rebuild moves behind an explicit
gate on `AgentSession`.

```
                    ┌─────────────────────────────┐
  ContentDelta ───► │ pending_text / messages     │  always
  ReasoningDelta ─► │ chat_ui_dirty = true        │
  Tool* / end ────► │ (+ structural materialize)  │
                    └──────────────┬──────────────┘
                                   │
              StreamTick ──────────┤  if chat_ui_dirty
              structural event ────┤
                                   ▼
                    materialize_chat_ui(ax, highlighter)
                                   │
                                   ▼
                    chat_blocks / chat_editors  (view input)
```

```rust
// area/interaction.rs — AgentSession gains a transient dirty flag.
pub struct AgentSession {
    // … existing fields …
    /// True when `session` transcript may have changed since last
    /// `materialize_chat_ui`. Cleared by materialize. Not persisted.
    pub chat_ui_dirty: bool,
}

/// Rebuild chat blocks/editors from `ax.session`. Clears `chat_ui_dirty`.
pub fn materialize_chat_ui(ax: &mut AgentSession, highlighter: &SyntaxHighlighter) {
    rebuild_chat_editor(ax, highlighter); // existing entry; may grow helpers inside
    ax.chat_ui_dirty = false;
}

/// Whether this agent event must paint immediately (not wait for StreamTick).
fn is_structural_chat_event(evt: &agent::AgentEvent) -> bool {
    todo!("ToolUse | ToolResult | TurnComplete | Error | ProcessExited | …")
}
```

```rust
// main.rs — sketch of the gate (names only).
Message::AgentEvent(key, evt) => {
    // apply session mutations as today…
    ax.chat_ui_dirty = true; // when transcript body / pending buffers change
    if is_structural_chat_event(&evt) || !ax.session.is_streaming {
        interaction::materialize_chat_ui(ax, highlighter);
    }
    // else: StreamTick will materialize while dirty + streaming
}

Message::StreamTick => {
    widget::streaming_indicator::bump_tick();
    // for each streaming session with chat_ui_dirty:
    //   materialize_chat_ui + optional stick_to_bottom snap
}
```

**Invariants.**

- `pending_text` / `pending_reasoning` always update on the event that carries them —
  never gated.

- Persistence (`needs_flush`, `FlushTick`, turn-boundary save) is independent of
  `chat_ui_dirty`.

- Non-streaming paths (load session, send prompt, recover) still materialize immediately.

- At most one materialize per `StreamTick` per session for pure text/reasoning streams;
  structural events may materialize in the same event turn without waiting.

## Live answer editor refresh

`rebuild_chat_editor` already reuses an editor when `chat_blocks[i].lines == block.lines`.
The live answer almost never hits that path during a stream — content grows every delta —
so today it always does:

```
EditorState::new(joined) + highlight_lines(all lines, md syntax)
```

That is the second main cost (after hybrid layout) on the growing block.

```
settled block i     live block n
──────────────     ─────────────
lines equal?          lines grew
  reuse editor          │
                        ▼
              same block index + same kind?
                 │              │
                yes            no (new Activity, etc.)
                 │              │
                 ▼              ▼
          in-place refresh   full EditorState::new
          append lines       + full highlight
          partial highlight
```

```rust
// area/interaction.rs — inside rebuild_chat_editor (sketch)
fn refresh_live_editor(
    existing: &mut EditorState,
    new_lines: &[String],
    highlighter: &SyntaxHighlighter,
) {
    // When only a suffix of lines changed (typical stream append):
    //   Arc::make_mut(&mut existing.lines); extend / rewrite tail
    //   extend highlight_spans for new lines; re-highlight dirty line range
    //   bump highlight_version
    // Fallback: replace with EditorState::new + full highlight (structural reshape)
    todo!()
}
```

**Rules of thumb for the implementation.**

- Prefer suffix-append detection: same prefix lines, last line may grow, then new lines.

- Re-highlight only the dirty line range when syntect line highlighting is independent
  enough for markdown; if a prefix re-highlight is required for correctness, still avoid
  rebuilding the whole `EditorState` (cursor/scroll/selection identity on the block).

- Block count or kind changes (Activity insert, Thinking ↔ Answer switch after flush) use
  the full path for affected indices; settled earlier blocks still reuse by line equality.

## Shared hybrid layout cache

`cached_hybrid_layout` today stores `(HybridLayoutKey, EditorLayout)` and **returns
`ed.clone()` on hit**. `EditorLayout` owns `TableLayout` (regions → rows → cells →
fragments). With large tables, clone-on-hit dominates even when layout is valid — and
`layout`, `update`, and `draw` each take a hit per frame.

```
today                         target
─────                         ──────
cache: EditorLayout           cache: Arc<EditorLayout>
hit → deep clone              hit → Arc::clone (refcount)
miss → compute; store clone   miss → compute; Arc::new; store
```

```rust
// widget/text_edit/render.rs
struct InternalState {
    // …
    hybrid_layout: RefCell<Option<(HybridLayoutKey, Arc<EditorLayout>)>>,
}

fn cached_hybrid_layout(
    internal: &InternalState,
    lines: &Arc<Vec<String>>,
    highlight_version: u64,
    pane_chars: usize,
    word_wrap: bool,
) -> Arc<EditorLayout> {
    let key = HybridLayoutKey::new(lines, highlight_version, pane_chars, word_wrap);
    let mut cache = internal.hybrid_layout.borrow_mut();
    if let Some((k, ed)) = cache.as_ref()
        && *k == key
    {
        return Arc::clone(ed);
    }
    let ed = Arc::new(EditorLayout::compute(lines, pane_chars, word_wrap));
    *cache = Some((key, Arc::clone(&ed)));
    ed
}
```

Call sites in `TextEdit::layout` / `update` / `draw` take `&EditorLayout` via
`ed.as_ref()` (or keep the `Arc` for the duration of the method). Invalidation keys stay
the same: `pane_chars`, `word_wrap`, `lines` Arc ptr, `highlight_version`, `line_count`.

**Out of scope for the pure kernel.** `md_table::layout_tables` still scans the buffer and
returns owned `TableLayout`. Callers that re-layout every materialize still pay full
compute for the live answer — that is acceptable once materialize is ~10 Hz and settled
blocks hit the shared cache without cloning.

Optional follow-up (not required for this design): paint without per-fragment
`String::collect` (slice into cell text). Only if profiling still shows draw cost after
the above.

## Structural immediacy

Some events change transcript *shape*, not just the open Answer/Thinking tail. Waiting a
tick would briefly show wrong structure (missing tool card, stale Thinking after flush).

```
| Event class | Session apply | Materialize |
|---|---|---|
| `ContentDelta` / `ReasoningDelta` | always | dirty → next `StreamTick` |
| Kind switch that flushes the other pending buffer | always | **immediate** (same event) |
| `ToolUse` / `ToolResult` | always | **immediate** |
| `TurnComplete` / `Error` / `ProcessExited` | always | **immediate** |
| Load / send / recover (non-stream paths) | n/a | **immediate** |
```

Kind switch is already explicit in `main` (`flush_pending_reasoning` before answer delta,
and vice versa). After that flush, materialize immediately so the new segment appears even
if the next pure-delta batch is coalesced.

Stick-to-bottom snap continues to run after materialize when `stick_to_bottom` is set —
same as today after `rebuild_chat_editor` on `AgentEvent`, and also after tick-driven
materialize so auto-scroll still tracks the live answer without per-token scroll tasks.

## Decisions

- **Coalesce on existing `StreamTick` (~100 ms)** — reuses the streaming-only subscription
  and matches the indicator animation cadence. Alternatives: per-delta rebuild (status
  quo; rejected — freezes under tables); separate 16 ms / 50 ms timer (rejected — extra
  wakeups without a clear win over 10 Hz); coalesce only when tables present (rejected —
  highlight cost is independent of tables; one policy is simpler).

- **In-place live editor refresh with partial highlight** — keep block identity and avoid
  full syntect of the whole answer every tick. Alternatives: always `EditorState::new`
  (rejected — rebuilds Arc lines + full highlight every materialize); defer all
  highlighting until `TurnComplete` (rejected — long answers stay unhighlighted for the
  whole turn).

- **`Arc<EditorLayout>` in the hybrid cache** — cheap hit path for settled table-heavy
  blocks. Alternatives: deep clone (status quo; rejected); `RefCell` borrow API that never
  returns owned layout (rejected — `layout`/`update`/`draw` lifetimes and nested borrows
  are awkward with iced’s call pattern); store layout on `EditorState` (rejected — couples
  pure-ish state to pane width and forces more invalidation plumbing).

- **No chat list virtualization in this change** — proposal out of scope; coalesce + reuse
  + shared layout should reclaim most mid-stream freezes without an iced
  scroll-architecture rewrite. Revisit if long *settled* histories remain janky after this
  lands.

## Risks

- **Coalesced answer feels “chunky” (~10 updates/s)** → Acceptable for agent streams; if
  it feels laggy, lower interval toward 50 ms without returning to per-delta rebuild.
  Final `TurnComplete` always paints the full answer immediately.

- **Partial highlight wrong on markdown state that spans lines** → Fallback to
  re-highlight from a safe line (or full buffer) when the dirty range is large or spans
  fence/table boundaries; never ship a path that leaves stale spans after `TurnComplete`.

- **Missed materialize leaves UI behind session** → Every structural path and every
  non-streaming entry point calls materialize; `StreamTick` drains dirty while streaming;
  debug assert or tracing when `is_streaming && chat_ui_dirty` across many ticks without
  drain is optional hardening.

## Open questions

None — interval, live refresh, and shared layout cache are decided above. Spec can turn
`chat/stream-ui` into requirements on dirty/materialize timing, settled reuse, and
no-clone hybrid cache behavior without further design choices.
