# Freeform send while awaiting

When the session is awaiting a user choice, freeform composer submit cancels the pending
choice harness-normally, clears chips, and sends that text as a user turn — not interrupt-
queue staging that needs a second confirm.

## Prerequisites

- [x] @step drop-cancel-chrome

## Context

`SendPressed` today treats any streaming turn (including awaiting-choice) as queue-or-
interrupt. Spec: complete pending as `UserChoiceAnswer::Cancelled` first so Claude/Grok
get deny / skip_interview, then dispatch freeform without leaving text only in the queue.

## Tasks

- [x] 1. In `SendPressed` (and equivalent send path) in
         `crates/duckboard/src/area/interaction.rs`: if `is_awaiting_user` and non-empty
         freeform text, answer pending choice as cancelled, clear shell, then send the
         text as a user turn (not leave it only in the interrupt queue)

- [x] 2. Ensure ordinary streaming queue/interrupt behavior is unchanged when not awaiting
         a choice
