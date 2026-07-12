# Dial back last-Answer band

Soften `bg_chat_last_answer` so the full-width band is clearly above the chat area but
less loud than the step-05 elevated mix.

## Context

From `reviews/02-followup-band-contrast-tune.md`: landmarks OK; current ~55% surface + 45%
elevated is too strong. Prefer a mid setting (e.g. full `bg_surface` or a milder
surface/elevated mix).

## Tasks

- [x] 1. Soften `bg_chat_last_answer` in `crates/duckboard/src/theme.rs` toward full
         `bg_surface` (or a mild mix still below the step-05 elevated punch)

- [x] 2. Smoke: band still readable as “latest Answer” in light/dark, not card chrome, not
         as heavy as before

## Outcomes

- Band is plain `bg_surface()` — between the original half-step and the elevated mix.
