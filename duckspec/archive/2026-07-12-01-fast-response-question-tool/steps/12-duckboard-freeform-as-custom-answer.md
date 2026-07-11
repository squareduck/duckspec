# Duckboard freeform as custom answer

Composer submit while awaiting completes the pending choice as `Custom` freeform text
(not Cancelled + interrupt queue). Clear the shell; no second-confirm freeform path.

## Prerequisites

- [x] @step custom-freeform-answer-wire

## Tasks

- [x] 1. Freeform plan / `SendPressed` in `crates/duckboard/src/area/interaction.rs`: answer
         `UserChoiceAnswer::Custom { text }`, clear shell; do not stage freeform only in
         the interrupt queue or hard-cancel the turn for freeform

- [x] 2. Remove obsolete freeform `@spec` tasks from steps
         `09-freeform-send-while-awaiting.md` and
         `10-freeform-cancel-without-hard-interrupt.md` (old cancel-and-send scenario name)

- [x] 3. @spec chat/fast-response Freeform while awaiting: Freeform submit completes the pending choice as a custom answer
