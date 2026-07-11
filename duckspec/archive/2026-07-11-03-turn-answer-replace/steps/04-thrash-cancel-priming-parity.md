# Thrash cancel priming parity

On thrash trip, clear priming/follow-up state the same way as CancelPressed so
TurnComplete cannot dispatch a staged follow-up after thrash cancel.

## Context

Review finding 1: thrash path in `main.rs` only calls `handle.cancel()`; CancelPressed
also sets `priming_in_flight = false` and `pending_followup_prompt = None`.

## Tasks

- [x] 1. On thrash trip (ContentDelta path in `main.rs`, after `on_answer_thrash_trip` /
         `handle.cancel()`), clear `priming_in_flight` and `pending_followup_prompt` to
         match CancelPressed — preferably via a small shared helper if that stays cleaner
         than duplicating two assignments

- [x] 2. Smoke-check: thrash trip still flushes last draft, appends stop notice, and
         cancels; no new follow-up send is staged from a cleared priming path
