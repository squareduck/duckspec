# Settings global vs project and oneshot rows

Restructure Settings into global vs this-project sections: concrete global default picker,
project override with “Use global default”, and oneshot rows driven by non-empty catalog
harnesses.

## Prerequisites

- [x] @step global-default-config-and-seed

## Tasks

- [x] 1. Restructure `crates/duckboard/src/area/settings.rs` view into Global (fonts,
         default model, chat) and This project (when root open) sections

- [x] 2. Add `GlobalModelSelected` writing a concrete `ModelRef` via
         `set_global_model_default`; choices from catalog only (no sentinel)

- [x] 3. Rename project picker copy to “Use global default” for the clear-override
         sentinel; keep `ModelDefaultSelected` clearing project map entry

- [x] 4. Replace `ONESHOT_HARNESS_ORDER` iteration with harnesses that have non-empty
         process-catalog slices (catalog harness order)

- [x] 5. Add `global_model_choices` / `project_override_model_choices` helpers in
         `crates/duckboard/src/widget/agent_chat.rs` if settings still needs them
