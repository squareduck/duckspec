# Stronger last-Answer band

Raise full-width last-Answer contrast so the latest reply is easier to spot, without card
chrome.

## Prerequisites

- [x] @step landmark-scroll-and-streaming

## Context

From `reviews/01-followup-landmark-scroll-and-band-contrast.md` issue 3: half-mix
`bg_chat_area` → `bg_surface` is too subtle.

## Tasks

- [x] 1. Strengthen `bg_chat_last_answer` in `crates/duckboard/src/theme.rs` (e.g. full
         `bg_surface` or a step toward elevated / quiet accent) for light and dark

- [x] 2. Smoke: latest Answer band is clearly more contrasty than older Answers, still
         full-width not a card

## Outcomes

- Band is ~55% `bg_surface` + 45% `bg_elevated` (was half-step from `bg_chat_area` toward
  surface only). Still borderless full-width. Eyeball in light/dark if needed.
