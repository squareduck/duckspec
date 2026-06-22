# Current review in session orientation

Load a change's reviews into duckboard and surface the current (highest-`NN`) review in
the session orientation, without affecting phase or next-stage.

## Tasks

- [x] 1. Add a `reviews: Vec<String>` field to `ChangeData` in
         `crates/duckboard/src/data.rs` and populate it from the change's `reviews/`
         directory (sorted), alongside where `steps` is loaded.

- [x] 2. Add a `current_review: Option<String>` field to `ChangeScopeFacts` in
         `crates/duckboard/src/area/change.rs`, computed as the highest-`NN` review and
         set in every return arm of `change_scope_facts`; leave `phase` and `next_command`
         untouched.

- [x] 3. Render the current-review sentence in `render_change_orientation` in
         `crates/duckboard/src/scope.rs`, appended only when `current_review` is present.

- [x] 4. @spec session/scope Current review in orientation: Orientation reports the highest-numbered review as the current review

- [x] 5. @spec session/scope Current review in orientation: A change with no reviews reports no current review

- [x] 6. @spec session/scope Current review in orientation: Adding a review does not change the suggested next stage
