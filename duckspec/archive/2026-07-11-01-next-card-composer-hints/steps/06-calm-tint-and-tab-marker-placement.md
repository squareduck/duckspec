# Calm tint and tab marker placement

Quieter meta-card backgrounds for both themes; sit the tab-available marker with the
next-action ghost, not as a strip under the input.

## Prerequisites

- [x] @step transcript-meta-card-tint
- [x] @step next-action-composer

## Context

From followup `reviews/01-followup-composer-polish.md`: tryout found `META_CARD_BG` too
loud and the `⇥` marker as a detached full-width row under the composer.

## Tasks

- [x] 1. Soften `META_CARD_BG` for macchiato and latte (lower chroma, closer to
         surface/mantle); keep distinct from search Match and diff hunks

- [x] 2. Reposition the tab-available marker so it sits with the empty-composer next
         affordance (ghost), not as a separate full-width under-input strip; keep
         visibility rules (`len > 1`, empty input, idle)
