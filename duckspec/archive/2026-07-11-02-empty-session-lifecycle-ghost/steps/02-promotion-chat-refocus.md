# Promotion chat refocus

After a bound exploration→change promotion, restore chat input focus so the user can keep
typing without re-clicking; unbound new directories must not force focus.

## Prerequisites

- [x] @step scope-lifecycle-bootstrap

## Tasks

- [x] 1. Change `promote_bound_exploration` to return `bool` (true when a binding was
         consumed and promotion ran)

- [x] 2. Replace `reload_and_reconcile`'s bare `bool` with a small outcome (`archived` +
         `promoted`); update file-watcher and `Message::Refresh` call sites

- [x] 3. When `promoted` is true, batch `focus_chat_input()` with any existing follow-up
         tasks from those callers

- [x] 4. @spec exploration/promotion Chat focus after bound promotion: Bound promotion restores chat input focus

- [x] 5. @spec exploration/promotion Chat focus after bound promotion: Unbound new change does not force chat input focus
