# Duckboard reviews section

Surface a change's reviews in the duckboard change UI as a new collapsible "Reviews"
section, placed after Capabilities, with each review a selectable row that opens in the
content pane.

## Context

The duckboard change view renders Overview, Capabilities, Steps, and Changed Files
(`view_list` in `crates/duckboard/src/area/change.rs:1063`), but `reviews/` files —
already loaded into `ChangeData.reviews` (`data.rs`) and surfaced in the session
orientation — have no UI. This step adds the missing section so reviews created by
`/ds-review` are browsable, mirroring how `view_steps_section` renders steps.

Section ordering, per the request: Overview, Capabilities, **Reviews**, Steps, Changed
Files. This is UI plumbing — duckboard rendering carries no capability spec (matching the
existing Steps/Capabilities sections), so there are no `@spec` tasks.

A review row's selectable id is its project-relative path
(`{change.prefix}/reviews/{filename}`), exactly as Overview and Steps build their ids; the
existing content-pane loader resolves an id to a file path, so a review opens with no new
loader code — task 5 verifies this.

## Tasks

- [x] 1. Add a `view_reviews_section` function to `crates/duckboard/src/area/change.rs`,
         mirroring `view_steps_section`: one `ListRow` per entry in `change.reviews`, each
         with id `format!("{}/reviews/{}", change.prefix, filename)`, selected/errored
         state wired the same way, and a `collapsible::view` titled "Reviews" keyed on the
         `"reviews"` section. Render the empty state ("No reviews") via `list_view::view`.

- [x] 2. Call `view_reviews_section` in `view_list` immediately after `view_caps_section`
         and before `view_steps_section` (`area/change.rs:1064-1065`).

- [x] 3. Insert `"reviews"` into the default `expanded_sections` set in `State::new`
         (`area/change.rs:87-92`) so the section starts expanded like the others.

- [x] 4. Add a `reviews/` arm to `parse_change_inner` (`area/change.rs:907`) returning
         `vec!["Reviews".into(), rest.into()]`, so a selected review's breadcrumb shows a
         `Reviews` segment ahead of the filename, mirroring the `steps/` arm. Give review
         rows a sensible icon (reuse `ICON_DOC` / `icon_for_artifact`, or add a dedicated
         review icon).

- [x] 5. Build and run duckboard; confirm a change with reviews shows the Reviews section
         after Capabilities, that selecting a review opens its rendered markdown in the
         content pane, and that a change with no reviews shows the empty state. Run
         `cargo test` to confirm the suite stays green.
