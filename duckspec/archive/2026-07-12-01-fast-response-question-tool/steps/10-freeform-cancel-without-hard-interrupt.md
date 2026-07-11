# Freeform cancel without hard interrupt

Stop freeform-while-awaiting from killing the agent process after a harness-normal
choice cancel. Complete the parked choice as cancelled, arm freeform for
`TurnComplete` auto-flush, and let the open turn finish so Grok can accept
`skip_interview` (Claude keep working).

## Context

Followup `reviews/03-followup-grok-freeform-hang.md`: Claude freeform works; Grok hangs
when freeform does answer `Cancelled` + queue + immediate `handle.cancel()`. Spec still
requires cancel pending + send freeform without a second confirm — queue flush on natural
turn end is enough; hard interrupt is not.

## Tasks

- [x] 1. In `plan_freeform_while_awaiting`
         (`crates/duckboard/src/area/interaction.rs`): set `interrupt_turn: false` while
         still returning cancel correlation id and armed queue text

- [x] 2. Keep `SendPressed` freeform branch: answer `Cancelled`, clear shell, arm queue;
         only call `handle.cancel()` when `interrupt_turn` is true (freeform no longer
         kills the process)
