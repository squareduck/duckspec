# Confirm gate reliability - Design

Two independent legs: duckboard captures the in-flight draft of any cancelled turn and
replays it to the agent as a system-reminder on the next send; the spec template folds the
first capability outline into the map turn so every later `confirm` lands on a
tool-anchored write turn.

## Approach

```
                     turn cancelled (user cancel │ thrash trip)
                                    │
                     pending_text non-empty at cancel?
                          │ yes                │ no
                          ▼                    ▼
              session.unsynced_draft = draft   nothing to sync
                     (persisted)               (committed text is already
                          │                     in the agent's history)
                          ▼
              next send_prompt_text on this session
                          │
        prompt = user text + <system-reminder> with the draft
                          │
              agent sees what the user saw → confirm lands right
```

Capture rule: only the **uncommitted pending draft** at cancellation. Text committed at
tool boundaries is already recorded by the agent runtime (verified against grok's
`chat_history.jsonl` for the `archive-list-ux` loop); the still-streaming draft is exactly
the divergent part.

## Unsynced draft capture (duckboard)

New persisted field on `ChatSession` (`crates/duckboard/src/chat_store.rs`):
`unsynced_draft: Option<String>`, serde-default, survives restart between cancel and next
send.

Capture inside `on_answer_thrash_trip` (before `flush_all_pending` clears `pending_text`)
and in the `CancelPressed` arm (`crates/duckboard/src/area/interaction.rs`) before
`handle.cancel()`.

```rust
/// Stash the in-flight draft at cancellation so the next send can resync it.
pub fn capture_unsynced_draft(session: &mut ChatSession) {
    if !session.pending_text.is_empty() {
        session.unsynced_draft = Some(session.pending_text.clone());
    }
}
```

## Resync reminder on next send (duckboard)

In `send_prompt_text` (`crates/duckboard/src/area/interaction.rs`): when `unsynced_draft`
is set, append a `<system-reminder>` block **after** the user's text (front-inlining
breaks slash-command parsing; `system_additions` only works on the first turn), then clear
the field. Wording tells the agent to treat the draft as its own already-sent reply and
respond only to the user's message.

## Spec template: map + first outline (duckspec)

`crates/duckspec/content/templates/spec.md`: the map turn ends with the first capability's
outline write gate; `confirm` approves map + outline together. Each write turn
creates/writes/formats/checks the confirmed capability, then presents the next outline
gate in the same (tool-anchored) turn. Handoff after the last write. Disagreement stays
freeform.

## Impact

- Old session JSONs load unchanged (optional field, serde default)
- `ds` reinstall required after the template edit (compile-time embedded content)

## Decisions

- **Capture scope** - pending draft only. Alternative: replay the whole turn (rejected:
  duplicates content the agent already has).

- **Delivery channel** - appended message-channel reminder. Alternatives:
  `system_additions` (first-turn-only), prepend (breaks slash commands).

- **Persistence** - on the session, not in memory. Alternative rejected: restart between
  cancel and send loses the resync.

- **Template shape** - fold first outline into map turn. Alternative: per-gate distinct
  confirm tokens (deferred; proposal non-goal).

- **Shutdown mid-turn needs no capture** - `pending_text` is in-memory only
  (`PersistedSession` omits it), so the draft vanishes from the user-visible transcript
  too; both sides consistently lack it and no divergence exists. The restart case that
  matters (cancel → quit → relaunch → send) is covered by persisting `unsynced_draft`.

## Risks

- **Agent answers the reminder instead of the user** → reminder wording is declarative
  context, explicitly "do not respond to this block".

- **Map + first-outline turn is still pure text and could thrash** → resync backstops it:
  even a cancelled gate turn resolves on the next confirm.
