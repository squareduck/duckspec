# Catalog ready UI wake

Notify iced when the app-start model catalog refresh finishes so pickers are not stuck
empty until an unrelated interaction.

## Prerequisites

- [x] @step process-model-catalog

## Context

Review `reviews/02-review-provider-model-discovery-post-implementation.md` finding 2:
`start_model_catalog_refresh` runs on a background thread with no iced message, so first
paint can keep empty model/oneshot pickers until some other event re-renders.

## Tasks

- [x] 1. After `refresh_registered` on the background thread, post a message the app
         handles (e.g. `Message::ModelCatalogReady`)

- [x] 2. On that message, apply a no-op (or catalog-aware) state update so
         view/subscriptions re-read the catalog

- [x] 3. Confirm Settings and chat model pickers re-resolve after the wake (manual or
         light test of the message path)

## Outcomes

- One-shot iced subscription runs `refresh_model_catalog` via `spawn_blocking`, then emits
  `Message::ModelCatalogReady` (process-wide once).

- Handler logs catalog size and returns `Task::none()` so iced re-renders and agent
  subscriptions re-resolve oneshot preferred from the filled catalog.
