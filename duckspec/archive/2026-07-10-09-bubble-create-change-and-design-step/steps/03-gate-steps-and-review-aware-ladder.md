# Gate, steps, and review-aware ladder

Implement review-priority lifecycle arms, the narrowed Confirm gate, and orientation next
stage / progress scenarios that share `change_scope_facts`.

## Prerequisites

- [x] @step compose-exploration-design-and-caps-ladder

## Tasks

- [x] 1. In `change_scope_facts`, order arms: open+review → apply only; open →
         apply+review; no-open+review → step+spec+archive; all-done no review →
         archive+review; then pre-step rungs

- [x] 2. In `build_obvious_chrome`, Confirm+Reject when nonempty and (has review or no
         steps); open steps without review stay lifecycle-only

- [x] 3. Update existing gate / all-complete / open-steps tests for the new rules

- [x] 4. @spec chat/obvious-bubble Chrome composition: Nonempty change session includes Confirm and Reject

- [x] 5. @spec chat/obvious-bubble Chrome composition: Open steps yield apply then review without gate

- [x] 6. @spec chat/obvious-bubble Chrome composition: Open steps with review yield apply only with gate

- [x] 7. @spec chat/obvious-bubble Chrome composition: No open steps with review yield step then spec then archive with gate

- [x] 8. @spec session/scope Lifecycle reflection: A change with all steps complete reports completion and the archive next-stage

- [x] 9. @spec session/scope Lifecycle reflection: All steps complete with a review suggests the step next-stage

- [x] 10. @spec session/scope Current review in orientation: Adding a review does not change reported step progress
