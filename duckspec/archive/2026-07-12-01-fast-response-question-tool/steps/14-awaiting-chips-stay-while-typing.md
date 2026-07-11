# Awaiting chips stay while typing

Keep option chips visible while awaiting a user choice even when the composer is non-empty
(custom-answer typing). When not awaiting, non-empty composer still hides chips. Fix the
obsolete step-01 `@spec` name for the renamed scenario.

## Context

Followup `reviews/05-followup-awaiting-chips-and-clippy.md` issue 1. Composer is the custom
answer surface while awaiting; hiding chips on first keystroke removes ⌘n mid-type.

## Tasks

- [x] 1. Update `visible` in `crates/duckboard/src/fast_response.rs`: require empty input
         only when `!is_awaiting_user`

- [x] 2. Update unit tests / call sites that assumed empty-input-only for chip visibility

- [x] 3. Fix obsolete `@spec` in `steps/01-rename-shell-to-fast-response.md` (old “Non-empty
         composer hides chips” name)

- [x] 4. @spec chat/fast-response Visibility: Non-empty composer hides chips when not awaiting

- [x] 5. @spec chat/fast-response Visibility: Awaiting user shows chips with non-empty composer
